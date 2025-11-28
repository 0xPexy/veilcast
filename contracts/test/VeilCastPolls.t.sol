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

    function _ffiProof(uint256 commitment, uint256 nullifier, uint256 pollId, uint256 membershipRoot)
        internal
        returns (bytes memory)
    {
        string[] memory cmds = new string[](7);
        cmds[0] = "python3";
        cmds[1] = "-c";
        cmds[2] =
            "import hashlib,sys\nc=int(sys.argv[1])\nn=int(sys.argv[2])\np=int(sys.argv[3])\nm=int(sys.argv[4])\ndata=c.to_bytes(32,'big')+n.to_bytes(32,'big')+p.to_bytes(32,'big')+m.to_bytes(32,'big')\nsys.stdout.write('0x'+hashlib.sha256(data).hexdigest())";
        cmds[3] = vm.toString(commitment);
        cmds[4] = vm.toString(nullifier);
        cmds[5] = vm.toString(pollId);
        cmds[6] = vm.toString(membershipRoot);
        bytes memory out = vm.ffi(cmds);
        if (out.length == 32) {
            return out;
        }
        return vm.parseBytes(string(out));
    }

    function testRevealHappyPath() external {
        (uint256 pollId,, uint256 revealEnd) = _createPoll();
        uint256 commitment = 111;
        uint256 nullifier = 222;
        bytes32[] memory pubInputs = new bytes32[](4);
        pubInputs[0] = bytes32(commitment);
        pubInputs[1] = bytes32(nullifier);
        pubInputs[2] = bytes32(pollId);
        pubInputs[3] = bytes32(uint256(1234));
        bytes memory proof = _ffiProof(commitment, nullifier, pollId, 1234);

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
        bytes32[] memory pubInputs = new bytes32[](4);
        pubInputs[0] = bytes32(commitment);
        pubInputs[1] = bytes32(nullifier);
        pubInputs[2] = bytes32(pollId);
        pubInputs[3] = bytes32(uint256(1234));
        bytes memory proof = _ffiProof(commitment, nullifier, pollId, 1234);
        vm.warp(revealEnd - 50);
        polls.reveal(pollId, 0, commitment, nullifier, proof, pubInputs);

        vm.expectRevert(VeilCastPolls.NullifierAlreadyUsed.selector);
        polls.reveal(pollId, 1, commitment, nullifier, proof, pubInputs);
    }

    function testRevealInvalidChoiceReverts() external {
        (uint256 pollId,, uint256 revealEnd) = _createPoll();
        uint256 commitment = 111;
        uint256 nullifier = 222;
        bytes32[] memory pubInputs = new bytes32[](4);
        pubInputs[0] = bytes32(commitment);
        pubInputs[1] = bytes32(nullifier);
        pubInputs[2] = bytes32(pollId);
        pubInputs[3] = bytes32(uint256(1234));
        bytes memory proof = _ffiProof(commitment, nullifier, pollId, 1234);
        vm.warp(revealEnd - 50);
        vm.expectRevert(VeilCastPolls.InvalidChoice.selector);
        polls.reveal(pollId, 2, commitment, nullifier, proof, pubInputs);
    }

    function testRevealWrongPhaseReverts() external {
        (uint256 pollId,, uint256 revealEnd) = _createPoll();
        uint256 commitment = 111;
        uint256 nullifier = 222;
        bytes32[] memory pubInputs = new bytes32[](4);
        pubInputs[0] = bytes32(commitment);
        pubInputs[1] = bytes32(nullifier);
        pubInputs[2] = bytes32(pollId);
        pubInputs[3] = bytes32(uint256(1234));
        bytes memory proof = _ffiProof(commitment, nullifier, pollId, 1234);
        vm.warp(revealEnd + 1);
        vm.expectRevert(VeilCastPolls.InvalidPhase.selector);
        polls.reveal(pollId, 0, commitment, nullifier, proof, pubInputs);
    }

    function testVerifierFailureReverts() external {
        (uint256 pollId,, uint256 revealEnd) = _createPoll();
        uint256 commitment = 111;
        uint256 nullifier = 222;
        bytes32[] memory pubInputs = new bytes32[](4);
        pubInputs[0] = bytes32(commitment);
        pubInputs[1] = bytes32(nullifier);
        pubInputs[2] = bytes32(pollId);
        pubInputs[3] = bytes32(uint256(1234));
        bytes memory badProof = hex"00";
        vm.warp(revealEnd - 10);
        vm.expectRevert(VeilCastPolls.VerifyFailed.selector);
        polls.reveal(pollId, 0, commitment, nullifier, badProof, pubInputs);
    }
}
