// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Script.sol";
import {HonkVerifier} from "../src/Verifier.sol";

/// @notice Deploys the Noir-generated Honk verifier.
/// Env:
///   RPC_URL=<sepolia/infura>
///   PRIVATE_KEY=<deployer pk>
contract DeployVerifier is Script {
    function run() external {
        vm.startBroadcast();
        HonkVerifier verifier = new HonkVerifier();
        vm.stopBroadcast();

        console2.log("Verifier deployed at", address(verifier));
    }
}
