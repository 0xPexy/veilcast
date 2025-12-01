// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Script.sol";
import {VeilCastPolls} from "../src/VeilCastPolls.sol";
import {IPollsVerifier} from "../src/VeilCastPolls.sol";

/// @notice Minimal deployment script for VeilCastPolls.
/// Env:
///   RPC_URL=<sepolia/infura>
///   PRIVATE_KEY=<deployer pk>
///   VERIFIER_ADDRESS=<deployed verifier>
contract Deploy is Script {
    function run() external {
        address verifier = vm.envAddress("VERIFIER_ADDRESS");

        vm.startBroadcast();
        VeilCastPolls polls = new VeilCastPolls(IPollsVerifier(verifier));
        vm.stopBroadcast();

        console2.log("VeilCastPolls deployed at", address(polls));
    }
}
