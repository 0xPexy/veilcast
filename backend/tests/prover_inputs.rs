use std::path::PathBuf;
use std::process::Command;

#[test]
fn noir_js_prover_inputs_pass() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = manifest_dir.join("scripts").join("run_prover_inputs.mjs");

    let output = Command::new("node")
        .arg(&script)
        .output()
        .expect("execute run_prover_inputs.mjs");

    assert!(
        output.status.success(),
        "run_prover_inputs failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("witness length"),
        "expected witness output, got: {}",
        stdout
    );
}
