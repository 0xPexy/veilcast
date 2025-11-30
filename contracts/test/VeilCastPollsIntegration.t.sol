// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../src/VeilCastPolls.sol";
import {HonkVerifier} from "../src/Verifier.sol";

contract VeilCastPollsIntegrationTest is Test {
    VeilCastPolls polls;
    HonkVerifier adapter;

    function setUp() external {
        adapter = new HonkVerifier();
        polls = new VeilCastPolls(IPollsVerifier(address(adapter)));
        vm.warp(1000);
    }

    function _createPoll(uint256 membershipRoot)
        internal
        returns (uint256 pollId, uint256 commitEnd, uint256 revealEnd)
    {
        commitEnd = block.timestamp + 100;
        revealEnd = block.timestamp + 200;
        string[] memory opts = new string[](2);
        opts[0] = "Yes";
        opts[1] = "No";
        pollId = polls.createPoll("Q", opts, commitEnd, revealEnd, membershipRoot);
    }

    /// @dev Calls node script to generate proof/publicInputs from zk/target/veilcast.json with fixed inputs.
    function _generateProofAndInputs(uint256 pollId, uint256 choice, uint256 secret, uint256 identitySecret)
        internal
        returns (bytes memory proof, bytes32[] memory pubInputs, uint256 membershipRoot)
    {
        string[] memory cmds = new string[](6);
        cmds[0] = "node";
        cmds[1] = string.concat(vm.projectRoot(), "/test/scripts/generate_proof.js");
        cmds[2] = vm.toString(pollId);
        cmds[3] = vm.toString(choice);
        cmds[4] = vm.toString(secret);
        cmds[5] = vm.toString(identitySecret);
        bytes memory out = vm.ffi(cmds);
        (proof, pubInputs, membershipRoot) = abi.decode(out, (bytes, bytes32[], uint256));
    }

    function testIntegrationReveal() external {
        // pollId we expect is 0 for first poll
        uint256 pollIdField = 0;
        uint256 choice = 1;
        uint256 secret = 42;
        uint256 identitySecret = 123;

        (bytes memory proof, bytes32[] memory publicInputs, uint256 membershipRoot) =
            _generateProofAndInputs(pollIdField, choice, secret, identitySecret);
        uint256 commitment = uint256(publicInputs[0]);
        uint256 nullifier = uint256(publicInputs[1]);
        (uint256 pollId,, uint256 revealEnd) = _createPoll(membershipRoot);

        vm.warp(revealEnd - 10);

        polls.reveal(pollId, uint8(choice), commitment, nullifier, proof, publicInputs);

        uint256[] memory counts = polls.getVotes(pollId);
        assertEq(counts[1], 1); // choice 1 incremented
    }

    function testIntegrationRejectsInvalidProof() external {
        uint256 pollIdField = 0;
        uint256 choice = 1;
        uint256 secret = 42;
        uint256 identitySecret = 123;

        (bytes memory proof, bytes32[] memory publicInputs, uint256 membershipRoot) =
            _generateProofAndInputs(pollIdField, choice, secret, identitySecret);
        uint256 commitment = uint256(publicInputs[0]);
        uint256 nullifier = uint256(publicInputs[1]);
        (uint256 pollId,, uint256 revealEnd) = _createPoll(membershipRoot);

        vm.warp(revealEnd - 5);

        // Tamper with proof (flip a byte) to force verifier failure.
        proof[0] = bytes1(uint8(proof[0]) ^ 0x01);
        vm.expectRevert(VeilCastPolls.VerifyFailed.selector);
        polls.reveal(pollId, uint8(choice), commitment, nullifier, proof, publicInputs);
    }
}
