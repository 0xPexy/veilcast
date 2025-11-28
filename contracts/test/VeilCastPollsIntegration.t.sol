// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../src/VeilCastPolls.sol";
import {HonkVerifier} from "../src/Verifier.sol";

contract VeilCastPollsIntegrationTest is Test {
    VeilCastPolls polls;
    HonkVerifier adapter;

    function setUp() external {
        console2.log("Setting up integration test...");
        adapter = new HonkVerifier();
        polls = new VeilCastPolls(IPollsVerifier(address(adapter)));
        vm.warp(1000);
    }

    function _createPoll(uint256 membershipRoot) internal returns (uint256 pollId, uint256 commitEnd, uint256 revealEnd) {
        commitEnd = block.timestamp + 100;
        revealEnd = block.timestamp + 200;
        string[] memory opts = new string[](2);
        opts[0] = "Yes";
        opts[1] = "No";
        pollId = polls.createPoll("Q", opts, commitEnd, revealEnd, membershipRoot);
    }

    /// @dev Calls node script to generate proof/publicInputs from zk/target/veilcast.json with fixed inputs.
    function _generateProofAndInputs(
        uint256 pollId,
        uint256 choice,
        uint256 secret,
        uint256 identitySecret
    ) internal returns (bytes memory proof, bytes32[] memory pubInputs, uint256 membershipRoot) {
        console2.log("Generating proof via FFI...");
        string[] memory cmds = new string[](6);
        cmds[0] = "node";
        cmds[1] = string.concat(vm.projectRoot(), "/test/scripts/generate_proof.js");
        cmds[2] = vm.toString(pollId);
        cmds[3] = vm.toString(choice);
        cmds[4] = vm.toString(secret);
        cmds[5] = vm.toString(identitySecret);
        console2.log("ffi");
        bytes memory out = vm.ffi(cmds);
        console2.log("ffi done");
        (proof, pubInputs, membershipRoot) = abi.decode(out, (bytes, bytes32[], uint256));
        console2.log("decode done");
        console2.log("Proof/publicInputs generated. membershipRoot:", membershipRoot);
    }

    function testIntegrationReveal() external {
        console2.log("Starting integration test reveal flow...");
        // pollId we expect is 0 for first poll
        uint256 pollIdField = 0;
        uint256 commitment = 0x10;
        uint256 nullifier = 0x20;
        uint256 choice = 1;
        uint256 secret = 42;
        uint256 identitySecret = 123;

        console2.log("Calling proof script...");
        (bytes memory proof, bytes32[] memory publicInputs, uint256 membershipRoot) =
            _generateProofAndInputs(pollIdField, choice, secret, identitySecret);
        // derive commitment/nullifier from public inputs
        commitment = uint256(publicInputs[0]);
        nullifier = uint256(publicInputs[1]);

        console2.log("Creating poll with membershipRoot...");
        (uint256 pollId,, uint256 revealEnd) = _createPoll(membershipRoot);

        console2.log("Warping to reveal window...");
        vm.warp(revealEnd - 10);

        console2.log("Revealing vote on-chain...");
        polls.reveal(pollId, uint8(choice), commitment, nullifier, proof, publicInputs);

        uint256[] memory counts = polls.getVotes(pollId);
        console2.log("Integration reveal success. Count[1]:", counts[1]);
        assertEq(counts[1], 1); // choice 1 incremented
    }
}
