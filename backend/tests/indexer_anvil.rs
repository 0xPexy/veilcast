use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use ethers::abi::{self, Abi, Token};
use ethers::contract::{Contract, ContractFactory};
use ethers::middleware::{Middleware, SignerMiddleware};
use ethers::providers::{Provider, Ws};
use ethers::signers::{LocalWallet, Signer};
use ethers::types::BigEndianHash;
use ethers::types::{Address, Bytes, Log, H256, U256};
use ethers::utils::Anvil;
use serde_json::Value;
use veilcast_backend::indexer;
use veilcast_backend::repo::{InMemoryStore, PollStore};

// Helper: load abi/bytecode from forge artifact JSON.
fn load_artifact(path: &Path) -> (Abi, Bytes) {
    let content = std::fs::read_to_string(path).unwrap_or_else(|_| {
        panic!(
            "artifact not found at {:?}. Run `forge build` in contracts.",
            path
        )
    });
    let v: Value = serde_json::from_str(&content).expect("invalid json");
    let abi: Abi = serde_json::from_value(v.get("abi").unwrap().clone()).expect("abi decode");
    let bytecode_val = v.get("bytecode").expect("missing bytecode");
    let bytecode_hex = if let Some(s) = bytecode_val.as_str() {
        s.to_string()
    } else {
        bytecode_val
            .get("object")
            .and_then(|o| o.as_str())
            .map(str::to_string)
            .expect("bytecode object string")
    };
    let bytecode =
        Bytes::from(hex::decode(bytecode_hex.trim_start_matches("0x")).expect("hex decode"));
    (abi, bytecode)
}

#[tokio::test]
async fn indexer_captures_poll_created_on_anvil() {
    // 1) Spawn local anvil
    let anvil = Anvil::new().spawn();
    let ws = Ws::connect(anvil.ws_endpoint()).await.unwrap();
    let provider = Provider::new(ws);
    let wallet: LocalWallet = anvil.keys()[0].clone().into();
    let wallet = wallet.with_chain_id(anvil.chain_id());
    let signer = SignerMiddleware::new(provider, wallet.clone());
    let client = Arc::new(signer);

    // 2) Load artifacts for VeilCastPolls (built with forge)
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../contracts/out");
    let polls_artifact = root.join("VeilCastPolls.sol/VeilCastPolls.json");
    let (polls_abi, polls_bytecode) = load_artifact(&polls_artifact);

    // Deploy VeilCastPolls with a dummy verifier address (our own)
    let factory = ContractFactory::new(polls_abi.clone(), polls_bytecode, client.clone());
    let polls_contract = factory
        .deploy(wallet.address())
        .expect("deploy args")
        .send()
        .await
        .expect("deploy send");
    let polls_addr = polls_contract.address();

    // 3) Prepare InMemory sink
    let store = Arc::new(InMemoryStore::default());

    // 4) Call createPoll to emit PollCreated
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let commit_end = now + 300;
    let reveal_end = commit_end + 600;
    Contract::new(polls_addr, polls_abi, client.clone())
        .method::<_, ()>(
            "createPoll",
            (
                String::from("Test Q"),
                vec![String::from("Yes"), String::from("No")],
                commit_end,
                reveal_end,
                1234u64,
            ),
        )
        .unwrap()
        .send()
        .await
        .expect("createPoll tx");

    // 5) Fetch logs and process via handle_log (simulating indexer)
    let logs = client
        .provider()
        .get_logs(&ethers::types::Filter::new().address(polls_addr))
        .await
        .expect("get_logs");
    for log in logs {
        indexer::handle_log(&store, log).await.expect("handle log");
    }
    let record = store.get_poll(0).await.expect("poll indexed");
    assert_eq!(record.question, "Test Q");
}

#[tokio::test]
async fn indexer_handles_vote_and_resolve_logs() {
    let store = Arc::new(InMemoryStore::default());
    let polls_addr = Address::random();

    // Feed PollCreated
    let created_log = make_poll_created_log(
        polls_addr,
        0,
        "Q2",
        vec!["Yes".into(), "No".into()],
        123,
        456,
        999,
    );
    indexer::handle_log(&store, created_log)
        .await
        .expect("poll created");

    // Feed VoteRevealed
    let vote_log = make_vote_revealed_log(polls_addr, 0, 1, 7777);
    indexer::handle_log(&store, vote_log)
        .await
        .expect("vote handled");

    // Feed PollResolved
    let resolved_log = make_poll_resolved_log(polls_addr, 0, 1);
    indexer::handle_log(&store, resolved_log)
        .await
        .expect("resolved");

    let updated = store.get_poll(0).await.expect("poll exists");
    assert!(updated.resolved);
    assert_eq!(updated.correct_option, Some(1));
}

fn make_poll_created_log(
    addr: Address,
    poll_id: u64,
    question: &str,
    options: Vec<String>,
    commit_end: u64,
    reveal_end: u64,
    membership_root: u64,
) -> Log {
    let sig = H256::from(ethers::utils::keccak256(
        "PollCreated(uint256,string,string[],uint256,uint256,uint256)",
    ));
    let topics = vec![sig, H256::from_uint(&U256::from(poll_id))];
    let data = abi::encode(&[
        Token::String(question.into()),
        Token::Array(options.into_iter().map(Token::String).collect()),
        Token::Uint(U256::from(commit_end)),
        Token::Uint(U256::from(reveal_end)),
        Token::Uint(U256::from(membership_root)),
    ]);
    Log {
        address: addr,
        topics,
        data: data.into(),
        ..Default::default()
    }
}

fn make_vote_revealed_log(addr: Address, poll_id: u64, choice_index: u8, nullifier: u64) -> Log {
    let sig = H256::from(ethers::utils::keccak256(
        "VoteRevealed(uint256,uint8,uint256)",
    ));
    let topics = vec![sig];
    let data = abi::encode(&[
        Token::Uint(U256::from(poll_id)),
        Token::Uint(U256::from(choice_index)),
        Token::Uint(U256::from(nullifier)),
    ]);
    Log {
        address: addr,
        topics,
        data: data.into(),
        ..Default::default()
    }
}

fn make_poll_resolved_log(addr: Address, poll_id: u64, correct_option: u8) -> Log {
    let sig = H256::from(ethers::utils::keccak256("PollResolved(uint256,uint8)"));
    let topics = vec![sig];
    let data = abi::encode(&[
        Token::Uint(U256::from(poll_id)),
        Token::Uint(U256::from(correct_option)),
    ]);
    Log {
        address: addr,
        topics,
        data: data.into(),
        ..Default::default()
    }
}
