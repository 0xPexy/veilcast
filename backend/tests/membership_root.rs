use num_bigint::BigUint;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::NamedTempFile;
use toml::Value;

#[derive(Debug, Deserialize)]
struct MerklePaths {
    root: String,
    paths: HashMap<String, MerkleEntry>,
}

#[derive(Debug, Deserialize)]
struct MerkleEntry {
    bits: Vec<String>,
    siblings: Vec<String>,
}

#[test]
#[ignore = "requires matching Prover.toml membership_root"]
fn membership_root_matches_prover_inputs() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = manifest_dir.join("scripts").join("poseidon_merkle.mjs");
    let prover_toml = manifest_dir.join("../zk/Prover.toml");

    let toml_str = fs::read_to_string(&prover_toml).expect("read Prover.toml");
    let value: Value = toml::from_str(&toml_str).expect("parse Prover.toml");

    let identity_secret = value["identity_secret"]
        .as_str()
        .expect("identity_secret in Prover.toml");
    let expected_dec = value["membership_root"]
        .as_str()
        .expect("membership_root in Prover.toml");
    let expected_hex = decimal_to_hex(expected_dec);

    let mut tmp = NamedTempFile::new().expect("tmp file");
    serde_json::to_writer(
        &mut tmp,
        &json!({
            "members": [identity_secret],
            "depth": 20
        }),
    )
    .expect("write payload");

    let output = Command::new("node")
        .arg(&script)
        .arg(tmp.path())
        .output()
        .expect("execute poseidon script");

    assert!(
        output.status.success(),
        "poseidon script failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let res: MerklePaths =
        serde_json::from_slice(&output.stdout).expect("decode poseidon result");
    assert_eq!(
        res.root, expected_hex,
        "root should match membership_root from Prover.toml"
    );

    let entry = res
        .paths
        .get(identity_secret)
        .expect("path for identity");
    assert_eq!(entry.bits.len(), 20);
    assert!(entry.bits.iter().all(|b| b == "0"));
    assert_eq!(entry.siblings.len(), 20);
    assert!(entry.siblings.iter().all(|s| s == "0"));
}

fn decimal_to_hex(dec_str: &str) -> String {
    let value =
        BigUint::parse_bytes(dec_str.as_bytes(), 10).expect("parse decimal membership_root");
    format!("0x{:064x}", value)
}
