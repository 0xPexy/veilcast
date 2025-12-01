include .env

NETWORK ?= anvil   # anvil | sepolia
SEPOLIA_RPC_URL ?= $(RPC_URL)
LOCAL_RPC_URL ?= http://127.0.0.1:8545
ETHERSCAN_API_KEY ?= $(ETHERSCAN_API_KEY)
ACCOUNT ?= default

default: help

help:
	@echo "Targets:"
	@echo "  deploy-verifier      Deploy HonkVerifier (uses --account)"
	@echo "  deploy-polls         Deploy VeilCastPolls with VERIFIER=<addr> (uses --account)"
	@echo "  verify-verifier      Etherscan verify deployed verifier"
	@echo "  verify-polls         Etherscan verify deployed VeilCastPolls"
	@echo "  gen-verifier         Generate contracts/src/Verifier.sol from ACIR via bb.js"
	@echo ""
	@echo "Env:"
	@echo "  NETWORK              anvil (default) | sepolia"
	@echo "  SEPOLIA_RPC_URL      RPC endpoint for sepolia (from .env RPC_URL)"
	@echo "  LOCAL_RPC_URL        RPC endpoint for anvil (default: http://127.0.0.1:8545)"
	@echo "  VERIFIER             Deployed verifier address (for deploy-polls)"
	@echo "  ACCOUNT              forge account name (default: default)"
	@echo "  ETHERSCAN_API_KEY    Required for verify-* targets"
	@echo "Usage:"
	@echo "  make deploy-verifier NETWORK=anvil ACCOUNT=name"
	@echo "  make deploy-polls VERIFIER=0x... NETWORK=sepolia SEPOLIA_RPC_URL=... ACCOUNT=name"
	@echo "  make verify-verifier ADDRESS=0x... ETHERSCAN_API_KEY=..."
	@echo "  make verify-polls ADDRESS=0x... VERIFIER=0x... ETHERSCAN_API_KEY=..."
	@echo "  make gen-verifier (uses node script/zk/generate_verifier_contract.mjs)"

deploy-verifier:
	$(eval RESOLVED_RPC=$(call rpc_for_network))
	@test -n "$(RESOLVED_RPC)" || (echo "RPC_URL required (set in .env or env var)"; exit 1)
	forge script script/DeployVerifier.s.sol:DeployVerifier \
		--rpc-url $(RESOLVED_RPC) \
		--account $(ACCOUNT) \
		--broadcast

deploy-polls:
	@test -n "$(VERIFIER)" || (echo "VERIFIER address required"; exit 1)
	$(eval RESOLVED_RPC=$(call rpc_for_network))
	@test -n "$(RESOLVED_RPC)" || (echo "RPC_URL required (set in .env or env var)"; exit 1)
	VERIFIER_ADDRESS=$(VERIFIER) forge script script/Deploy.s.sol:Deploy \
		--rpc-url $(RESOLVED_RPC) \
		--account $(ACCOUNT) \
		--broadcast

# Etherscan verification
verify-verifier:
	@test -n "$(ADDRESS)" || (echo "ADDRESS required"; exit 1)
	@test -n "$(ETHERSCAN_API_KEY)" || (echo "ETHERSCAN_API_KEY required"; exit 1)
	forge verify-contract \
		--chain 11155111 \
		--etherscan-api-key $(ETHERSCAN_API_KEY) \
		$(ADDRESS) \
		src/Verifier.sol:HonkVerifier

verify-polls:
	@test -n "$(ADDRESS)" || (echo "ADDRESS required"; exit 1)
	@test -n "$(VERIFIER)" || (echo "VERIFIER (constructor arg) required"; exit 1)
	@test -n "$(ETHERSCAN_API_KEY)" || (echo "ETHERSCAN_API_KEY required"; exit 1)
	forge verify-contract \
		--chain 11155111 \
		--etherscan-api-key $(ETHERSCAN_API_KEY) \
		$(ADDRESS) \
		src/VeilCastPolls.sol:VeilCastPolls \
		--constructor-args $(shell cast abi-encode "constructor(address)" $(VERIFIER))

gen-verifier:
	node script/zk/generate_verifier_contract.mjs

# Helper: choose RPC based on NETWORK
rpc_for_network = $(if $(filter $(NETWORK),anvil),$(LOCAL_RPC_URL),$(if $(filter $(NETWORK),sepolia),$(SEPOLIA_RPC_URL),))
