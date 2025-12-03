// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../src/VeilCastPolls.sol";

contract MockVerifier is IPollsVerifier {
    bool public allow = true;

    function setAllow(bool v) external {
        allow = v;
    }

    /// @dev Accept proof that matches sha256(commitment,nullifier,pollId,membershipRoot)
    function verify(bytes calldata proof, bytes32[] calldata publicInputs) external view override returns (bool) {
        if (publicInputs.length < 4) return false;
        bytes32 expected = sha256(abi.encode(publicInputs[0], publicInputs[1], publicInputs[2], publicInputs[3]));
        return allow && proof.length == 32 && bytes32(proof) == expected;
    }
}

contract VeilCastPollsTest is Test {
    VeilCastPolls polls;
    MockVerifier verifier;

    function setUp() external {
        verifier = new MockVerifier();
        polls = new VeilCastPolls(verifier);
        vm.warp(1000);
    }

    function _createPoll() internal returns (uint256 pollId, uint256 commitEnd, uint256 revealEnd) {
        commitEnd = block.timestamp + 100;
        revealEnd = block.timestamp + 200;
        string[] memory opts = new string[](2);
        opts[0] = "Yes";
        opts[1] = "No";
        pollId = polls.createPoll("Q", opts, commitEnd, revealEnd, 1234);
    }

    function _mockProof(uint256 commitment, uint256 nullifier, uint256 pollId, uint256 membershipRoot)
        internal
        pure
        returns (bytes memory proof, bytes32[] memory pubInputs)
    {
        pubInputs = new bytes32[](4);
        pubInputs[0] = bytes32(commitment);
        pubInputs[1] = bytes32(nullifier);
        pubInputs[2] = bytes32(pollId);
        pubInputs[3] = bytes32(membershipRoot);
        proof = abi.encodePacked(sha256(abi.encode(pubInputs[0], pubInputs[1], pubInputs[2], pubInputs[3])));
    }

    function testRevealHappyPath() external {
        (uint256 pollId,, uint256 revealEnd) = _createPoll();
        uint256 commitment = 111;
        uint256 nullifier = 222;
        (bytes memory proof, bytes32[] memory pubInputs) = _mockProof(commitment, nullifier, pollId, 1234);

        vm.warp(revealEnd - 50);
        polls.reveal(pollId, 0, commitment, nullifier, proof, pubInputs);

        uint256[] memory counts = polls.getVotes(pollId);
        assertEq(counts[0], 1);
        assertEq(counts[1], 0);
        assertTrue(polls.nullifierUsed(pollId, nullifier));
    }

    function testRevealDoubleVoteReverts() external {
        (uint256 pollId,, uint256 revealEnd) = _createPoll();
        uint256 commitment = 111;
        uint256 nullifier = 222;
        (bytes memory proof, bytes32[] memory pubInputs) = _mockProof(commitment, nullifier, pollId, 1234);
        vm.warp(revealEnd - 50);
        polls.reveal(pollId, 0, commitment, nullifier, proof, pubInputs);

        vm.expectRevert(VeilCastPolls.NullifierAlreadyUsed.selector);
        polls.reveal(pollId, 1, commitment, nullifier, proof, pubInputs);
    }

    function testRevealInvalidChoiceReverts() external {
        (uint256 pollId,, uint256 revealEnd) = _createPoll();
        uint256 commitment = 111;
        uint256 nullifier = 222;
        (bytes memory proof, bytes32[] memory pubInputs) = _mockProof(commitment, nullifier, pollId, 1234);
        vm.warp(revealEnd - 50);
        vm.expectRevert(VeilCastPolls.InvalidChoice.selector);
        polls.reveal(pollId, 2, commitment, nullifier, proof, pubInputs);
    }

    function testBatchRevealHappyPath() external {
        (uint256 pollId,, uint256 revealEnd) = _createPoll();
        uint256[] memory commitments = new uint256[](2);
        uint256[] memory nullifiers = new uint256[](2);
        uint8[] memory choices = new uint8[](2);
        bytes[] memory proofs = new bytes[](2);
        bytes32[][] memory pubInputs = new bytes32[][](2);

        commitments[0] = 111;
        commitments[1] = 222;
        nullifiers[0] = 333;
        nullifiers[1] = 444;
        choices[0] = 0;
        choices[1] = 1;

        (proofs[0], pubInputs[0]) = _mockProof(commitments[0], nullifiers[0], pollId, 1234);
        (proofs[1], pubInputs[1]) = _mockProof(commitments[1], nullifiers[1], pollId, 1234);

        vm.warp(revealEnd - 10);
        polls.batchReveal(pollId, choices, commitments, nullifiers, proofs, pubInputs);

        uint256[] memory counts = polls.getVotes(pollId);
        assertEq(counts[0], 1);
        assertEq(counts[1], 1);
        assertTrue(polls.nullifierUsed(pollId, nullifiers[0]));
        assertTrue(polls.nullifierUsed(pollId, nullifiers[1]));
    }

    function testBatchRevealLengthMismatch() external {
        (uint256 pollId,, uint256 revealEnd) = _createPoll();
        uint8[] memory choices = new uint8[](1);
        uint256[] memory commitments = new uint256[](2);
        uint256[] memory nullifiers = new uint256[](1);
        bytes[] memory proofs = new bytes[](1);
        bytes32[][] memory pubInputs = new bytes32[][](1);

        commitments[0] = 111;
        nullifiers[0] = 222;
        choices[0] = 0;
        (proofs[0], pubInputs[0]) = _mockProof(commitments[0], nullifiers[0], pollId, 1234);
        commitments[1] = 333;

        vm.warp(revealEnd - 5);
        vm.expectRevert(bytes("length mismatch"));
        polls.batchReveal(pollId, choices, commitments, nullifiers, proofs, pubInputs);
    }

    function testRevealWrongPhaseReverts() external {
        (uint256 pollId,, uint256 revealEnd) = _createPoll();
        uint256 commitment = 111;
        uint256 nullifier = 222;
        (bytes memory proof, bytes32[] memory pubInputs) = _mockProof(commitment, nullifier, pollId, 1234);
        vm.warp(revealEnd + 1);
        vm.expectRevert(VeilCastPolls.InvalidPhase.selector);
        polls.reveal(pollId, 0, commitment, nullifier, proof, pubInputs);
    }

    function testVerifierFailureReverts() external {
        (uint256 pollId,, uint256 revealEnd) = _createPoll();
        uint256 commitment = 111;
        uint256 nullifier = 222;
        (bytes memory proof, bytes32[] memory pubInputs) = _mockProof(commitment, nullifier, pollId, 1234);
        bytes memory badProof = hex"00";
        vm.warp(revealEnd - 10);
        vm.expectRevert(VeilCastPolls.VerifyFailed.selector);
        polls.reveal(pollId, 0, commitment, nullifier, badProof, pubInputs);

        verifier.setAllow(false);
        vm.expectRevert(VeilCastPolls.VerifyFailed.selector);
        polls.reveal(pollId, 0, commitment, nullifier, proof, pubInputs);
    }

    function testCreatePollValidation() external {
        string[] memory opts = new string[](1);
        opts[0] = "OnlyOne";
        vm.expectRevert(bytes("Need >=2 options"));
        polls.createPoll("Q", opts, block.timestamp + 10, block.timestamp + 20, 1);

        opts = new string[](2);
        opts[0] = "A";
        opts[1] = "B";
        vm.expectRevert(bytes("commit end past"));
        polls.createPoll("Q", opts, block.timestamp - 1, block.timestamp + 20, 1);

        vm.expectRevert(bytes("commit < reveal required"));
        polls.createPoll("Q", opts, block.timestamp + 20, block.timestamp + 10, 1);
    }

    function testCommitRecordsDuringCommit() external {
        (uint256 pollId, uint256 commitEnd,) = _createPoll();
        bytes32 commitment = keccak256("c");
        polls.commit(pollId, commitment);
        assertTrue(polls.seenCommitment(pollId, commitment));

        vm.warp(commitEnd + 1);
        vm.expectRevert(VeilCastPolls.InvalidPhase.selector);
        polls.commit(pollId, keccak256("late"));
    }

    function testResolveHappy() external {
        (uint256 pollId,, uint256 revealEnd) = _createPoll();
        uint256 commitment = 1;
        uint256 nullifier = 2;
        (bytes memory proof, bytes32[] memory pubInputs) = _mockProof(commitment, nullifier, pollId, 1234);
        vm.warp(revealEnd - 1);
        polls.reveal(pollId, 1, commitment, nullifier, proof, pubInputs);

        vm.warp(revealEnd + 1);
        polls.resolvePoll(pollId, 1);
        VeilCastPolls.Poll memory p = polls.getPoll(pollId);
        assertTrue(p.resolved);
        assertEq(p.correctOption, 1);
    }

    function testResolveValidation() external {
        (uint256 pollId,, uint256 revealEnd) = _createPoll();
        vm.warp(revealEnd - 1);
        vm.expectRevert(VeilCastPolls.InvalidPhase.selector);
        polls.resolvePoll(pollId, 0);

        vm.warp(revealEnd + 1);
        vm.expectRevert(VeilCastPolls.InvalidChoice.selector);
        polls.resolvePoll(pollId, 3);
    }

    function testGetPollInvalidReverts() external {
        vm.expectRevert(VeilCastPolls.InvalidPoll.selector);
        polls.getPoll(999);
    }
}
