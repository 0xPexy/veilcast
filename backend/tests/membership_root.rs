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
fn poseidon_merkle_runs_with_prover_identity() {
    // Read identity from Prover.toml to reuse current inputs.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let prover_toml = manifest_dir.join("../zk/Prover.toml");
    let toml_str = fs::read_to_string(&prover_toml).expect("read Prover.toml");
    let value: Value = toml::from_str(&toml_str).expect("parse Prover.toml");
    let identity_secret = value["identity_secret"]
        .as_str()
        .expect("identity_secret in Prover.toml");

    // Run noir_js-based poseidon merkle script.
    let script = manifest_dir
        .join("scripts")
        .join("poseidon_merkle_noir.mjs");
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

    let res: MerklePaths = serde_json::from_slice(&output.stdout).expect("decode poseidon result");

    // Basic sanity: root present and path length 20 with zero bits for single member.
    assert!(!res.root.is_empty(), "root should not be empty");
    let entry = res.paths.get(identity_secret).expect("path for identity");
    assert_eq!(entry.bits.len(), 20);
    assert!(entry.bits.iter().all(|b| b == "0"));
    assert_eq!(entry.siblings.len(), 20);
}
