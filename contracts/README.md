# VeilCast Contracts

## Core tasks
- Build: `forge build`
- Test: `forge test` (FFI integration tests: `FOUNDRY_FFI=1 forge test --ffi`)
- Format: `forge fmt`

## ZK scripts (bb.js)
- Install once: `cd contracts && npm install`
- Proof generation: `npm run proof -- <pollId> <choice> <secret> <identitySecret>`  
  (script: `script/zk/generate_proof.js`, uses `zk/target/veilcast.json`)
- Verifier generation: `npm run verifier`  
  (script: `script/zk/generate_verifier_contract.mjs` → `contracts/src/Verifier.sol`, keccak transcript)

## Deploy (Sepolia + Infura)
`.env` (not committed):
```
RPC_URL=https://sepolia.infura.io/v3/<INFURA_PROJECT_ID>
ETHERSCAN_API_KEY=<etherscan_key>  # optional (for verify)
```

1) Verifier deploy (if needed), using forge --account (configure via `forge account import`):
```bash
cd contracts
make deploy-verifier NETWORK=sepolia SEPOLIA_RPC_URL=$RPC_URL ACCOUNT=<forge_account_name>
```
2) VeilCastPolls deploy:
```bash
cd contracts
make deploy-polls VERIFIER=<verifier_address> NETWORK=sepolia SEPOLIA_RPC_URL=$RPC_URL ACCOUNT=<forge_account_name>
```
3) Etherscan verify (optional):
```bash
forge verify-contract \
  --chain 11155111 \
  --etherscan-api-key $ETHERSCAN_API_KEY \
  <DEPLOYED_ADDRESS> \
  src/VeilCastPolls.sol:VeilCastPolls \
  --constructor-args $(cast abi-encode "constructor(address)" $VERIFIER_ADDRESS) \
  --compiler-version v0.8.20
```
4) Update `infra/.env.backend` (`CONTRACT_ADDRESS`) and frontend `VITE_CONTRACT_ADDRESS` with the deployed address.

## Useful
- Anvil: `anvil`
- Cast: `cast <subcommand>`
- Foundry docs: https://book.getfoundry.sh/
