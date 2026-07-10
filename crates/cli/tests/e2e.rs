//! End-to-end CLI behaviour: offline sync, validation, evaluation, gating.

use assert_cmd::Command;

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn clearance() -> Command {
    let mut cmd = Command::cargo_bin("clearance").expect("binary builds");
    cmd.current_dir(repo_root());
    cmd
}

fn synced_snapshot(dir: &std::path::Path) -> std::path::PathBuf {
    let out = dir.join("snapshot.json");
    clearance()
        .args([
            "sync",
            "--governance",
            "content/governance/providers.yaml",
            "--aa-file",
            "crates/sync/tests/fixtures/aa-models.json",
            "--hf-file",
            "crates/sync/tests/fixtures/hf-models.json",
            "--generated-at",
            "2026-07-10T12:00:00Z",
            "--out",
        ])
        .arg(&out)
        .assert()
        .success();
    out
}

#[test]
fn validate_accepts_shipped_content_and_example_policy() {
    clearance()
        .args([
            "validate",
            "--rulebook",
            "content/rulebook/rulebook.yaml",
            "--policy",
            "examples/policy-no-us-cn-selfhost-ok.yaml",
            "--governance",
            "content/governance/providers.yaml",
        ])
        .assert()
        .success();
}

#[test]
fn validate_refuses_a_corrupt_policy() {
    let dir = std::env::temp_dir().join("clearance-e2e-corrupt");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let bad = dir.join("bad-policy.yaml");
    std::fs::write(
        &bad,
        "version: 1\nrules:\n  - id: org.x\n    severity: high\n",
    )
    .expect("write");

    clearance()
        .args([
            "validate",
            "--rulebook",
            "content/rulebook/rulebook.yaml",
            "--policy",
        ])
        .arg(&bad)
        .assert()
        .failure();

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn check_gates_on_the_canonical_scenario() {
    let dir = std::env::temp_dir().join("clearance-e2e-check");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let snapshot = synced_snapshot(&dir);

    // US-origin open-weight model, self-hostable → allowed under
    // "no US/CN data flow, self-host OK".
    clearance()
        .args([
            "check",
            "meta/llama-4",
            "--rulebook",
            "content/rulebook/rulebook.yaml",
            "--policy",
            "examples/policy-no-us-cn-selfhost-ok.yaml",
            "--need",
            "examples/need-code-public.yaml",
            "--snapshot",
        ])
        .arg(&snapshot)
        .assert()
        .success();

    // Closed model, US-jurisdiction API only → refused, exit code 1.
    clearance()
        .args([
            "check",
            "openai/gpt-6",
            "--rulebook",
            "content/rulebook/rulebook.yaml",
            "--policy",
            "examples/policy-no-us-cn-selfhost-ok.yaml",
            "--need",
            "examples/need-code-public.yaml",
            "--snapshot",
        ])
        .arg(&snapshot)
        .assert()
        .code(1);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn evaluate_ranks_eligible_models_as_json() {
    let dir = std::env::temp_dir().join("clearance-e2e-evaluate");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let snapshot = synced_snapshot(&dir);

    let assert = clearance()
        .args([
            "evaluate",
            "--json",
            "--rulebook",
            "content/rulebook/rulebook.yaml",
            "--policy",
            "examples/policy-no-us-cn-selfhost-ok.yaml",
            "--need",
            "examples/need-code-public.yaml",
            "--snapshot",
        ])
        .arg(&snapshot)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    let eligible = report["data"]["eligible"].as_array().expect("eligible[]");
    // Benchmarked Mistral ranks above the unbenchmarked HF-only Llama.
    assert_eq!(eligible[0]["model"], "mistralai/mistral-large-3");
    assert!(
        eligible
            .iter()
            .any(|entry| entry["model"] == "meta/llama-4")
    );
    assert!(
        !eligible
            .iter()
            .any(|entry| entry["model"] == "openai/gpt-6")
    );
    assert_eq!(report["data"]["ineligible_count"], 1);
    assert_eq!(report["data"]["indeterminate_count"], 1);
    assert_eq!(
        report["meta"]["snapshot_generated_at"],
        "2026-07-10T12:00:00Z"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn explain_reports_unknown_models_as_denied() {
    let dir = std::env::temp_dir().join("clearance-e2e-explain");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let snapshot = synced_snapshot(&dir);

    let assert = clearance()
        .args([
            "explain",
            "acme/never-heard-of",
            "--rulebook",
            "content/rulebook/rulebook.yaml",
            "--policy",
            "examples/policy-no-us-cn-selfhost-ok.yaml",
            "--need",
            "examples/need-code-public.yaml",
            "--snapshot",
        ])
        .arg(&snapshot)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    assert!(stdout.contains("builtin.unknown-model"), "got: {stdout}");

    std::fs::remove_dir_all(&dir).ok();
}
