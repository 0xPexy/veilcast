// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @notice Minimal verifier interface expected by polls.
interface IPollsVerifier {
    function verify(bytes calldata proof, bytes32[] calldata publicInputs) external view returns (bool);
}

/// @notice Simple ownable helper.
abstract contract Ownable {
    address public owner;

    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);

    constructor() {
        owner = msg.sender;
        emit OwnershipTransferred(address(0), msg.sender);
    }

    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }

    function transferOwnership(address newOwner) external onlyOwner {
        require(newOwner != address(0), "Zero address");
        emit OwnershipTransferred(owner, newOwner);
        owner = newOwner;
    }
}

/// @title VeilCastPolls
/// @notice Commit-reveal polling with optional zk verification and nullifier-based double-vote protection.
contract VeilCastPolls is Ownable {
    struct Poll {
        string question;
        string[] options;
        uint256 commitPhaseEnd;
        uint256 revealPhaseEnd;
        bool resolved;
        uint8 correctOption;
        uint256 membershipRoot;
    }

    IPollsVerifier public immutable verifier;
    uint256 public pollCount;
    mapping(uint256 => Poll) private polls;
    mapping(uint256 => mapping(bytes32 => bool)) public seenCommitment;
    mapping(uint256 => mapping(uint256 => bool)) public nullifierUsed;
    mapping(uint256 => mapping(uint8 => uint256)) public votes;

    event PollCreated(
        uint256 indexed pollId,
        string question,
        string[] options,
        uint256 commitPhaseEnd,
        uint256 revealPhaseEnd,
        uint256 membershipRoot
    );
    event Committed(uint256 indexed pollId, bytes32 commitment);
    event VoteRevealed(uint256 indexed pollId, uint8 choiceIndex, uint256 nullifier);
    event PollResolved(uint256 indexed pollId, uint8 correctOption);

    error InvalidPoll();
    error InvalidPhase();
    error InvalidChoice();
    error NullifierAlreadyUsed();
    error CommitmentUnknown();
    error VerifyFailed();

    constructor(IPollsVerifier _verifier) {
        verifier = _verifier;
    }

    /// @notice Create a new poll (owner-only).
    function createPoll(
        string calldata question,
        string[] calldata options,
        uint256 commitPhaseEnd,
        uint256 revealPhaseEnd,
        uint256 membershipRoot
    ) external onlyOwner returns (uint256 pollId) {
        require(options.length >= 2, "Need >=2 options");
        require(block.timestamp < commitPhaseEnd, "commit end past");
        require(commitPhaseEnd < revealPhaseEnd, "commit < reveal required");

        pollId = pollCount++;
        Poll storage p = polls[pollId];
        p.question = question;
        p.commitPhaseEnd = commitPhaseEnd;
        p.revealPhaseEnd = revealPhaseEnd;
        p.membershipRoot = membershipRoot;

        for (uint256 i = 0; i < options.length; i++) {
            p.options.push(options[i]);
        }

        emit PollCreated(pollId, question, options, commitPhaseEnd, revealPhaseEnd, membershipRoot);
    }

    /// @notice Optional on-chain commit recording.
    function commit(uint256 pollId, bytes32 commitment) external {
        Poll storage p = polls[pollId];
        if (!_pollExists(p)) revert InvalidPoll();
        if (block.timestamp >= p.commitPhaseEnd) revert InvalidPhase();
        seenCommitment[pollId][commitment] = true;
        emit Committed(pollId, commitment);
    }

    /// @notice Reveal a vote using a zk proof; increments aggregate tally.
    function reveal(
        uint256 pollId,
        uint8 choiceIndex,
        uint256 commitment,
        uint256 nullifier,
        bytes calldata proof,
        bytes32[] calldata publicInputs
    ) external {
        Poll storage p = polls[pollId];
        if (!_pollExists(p)) revert InvalidPoll();
        if (block.timestamp < p.commitPhaseEnd || block.timestamp >= p.revealPhaseEnd) revert InvalidPhase();
        if (choiceIndex >= p.options.length) revert InvalidChoice();
        if (nullifierUsed[pollId][nullifier]) revert NullifierAlreadyUsed();
        // Expecting 4 public inputs: commitment, nullifier, pollId, membershipRoot.
        if (publicInputs.length != 4) revert VerifyFailed();
        if (
            publicInputs[0] != bytes32(commitment) || publicInputs[1] != bytes32(nullifier)
                || publicInputs[2] != bytes32(pollId) || publicInputs[3] != bytes32(p.membershipRoot)
        ) revert VerifyFailed();

        nullifierUsed[pollId][nullifier] = true;

        bool ok = verifier.verify(proof, publicInputs);
        if (!ok) revert VerifyFailed();

        // Optional strict commit linkage: uncomment to enforce prior commit
        // if (!seenCommitment[pollId][bytes32(commitment)]) revert CommitmentUnknown();

        votes[pollId][choiceIndex] += 1;
        emit VoteRevealed(pollId, choiceIndex, nullifier);
    }

    /// @notice Resolve a poll by setting the correct option (owner-only).
    function resolvePoll(uint256 pollId, uint8 correctOption) external onlyOwner {
        Poll storage p = polls[pollId];
        if (!_pollExists(p)) revert InvalidPoll();
        if (block.timestamp < p.revealPhaseEnd) revert InvalidPhase();
        if (p.resolved) revert InvalidPhase();
        if (correctOption >= p.options.length) revert InvalidChoice();

        p.resolved = true;
        p.correctOption = correctOption;
        emit PollResolved(pollId, correctOption);
    }

    /// @notice View poll metadata.
    function getPoll(uint256 pollId) external view returns (Poll memory) {
        Poll storage p = polls[pollId];
        if (!_pollExists(p)) revert InvalidPoll();
        return p;
    }

    /// @notice Get vote counts per option.
    function getVotes(uint256 pollId) external view returns (uint256[] memory counts) {
        Poll storage p = polls[pollId];
        if (!_pollExists(p)) revert InvalidPoll();
        uint256 len = p.options.length;
        counts = new uint256[](len);
        for (uint256 i = 0; i < len; i++) {
            counts[i] = votes[pollId][uint8(i)];
        }
    }

    function _pollExists(Poll storage p) private view returns (bool) {
        return bytes(p.question).length != 0 || p.commitPhaseEnd != 0 || p.revealPhaseEnd != 0;
    }
}
