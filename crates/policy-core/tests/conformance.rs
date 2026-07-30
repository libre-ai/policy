// Conformance test against golden.json vectors
// SEMANTICS.md §10: An implementation conforms only if every vector and budget matches exactly.

use policy_core::evaluate;
use std::fs;

#[test]
fn test_conformance_golden_vectors() {
    // Load golden.json — resolved from the crate root (portable across machines/CI),
    // never a hardcoded absolute path.
    let golden_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/fixtures/policy-core-v2/golden.json"
    );
    let golden_content = fs::read_to_string(golden_path).expect("Failed to read golden.json");
    let golden: serde_json::Value =
        serde_json::from_str(&golden_content).expect("Failed to parse golden.json");

    let cases = golden
        .get("cases")
        .and_then(|c| c.as_array())
        .expect("No cases in golden.json");

    println!("Testing {} conformance vectors...", cases.len());
    let mut passed = 0;
    let mut failed = 0;

    for (i, test_case) in cases.iter().enumerate() {
        let case_id = test_case
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let policy = test_case.get("policy").expect("No policy in test case");
        let snapshot = test_case.get("snapshot").expect("No snapshot in test case");
        let need = test_case.get("need").expect("No need in test case");
        let evaluated_at = test_case
            .get("evaluatedAt")
            .and_then(|v| v.as_str())
            .expect("No evaluatedAt in test case");

        // Serialize inputs to JSON bytes
        let policy_bytes = serde_json::to_vec(policy).expect("Failed to serialize policy");
        let snapshot_bytes = serde_json::to_vec(snapshot).expect("Failed to serialize snapshot");
        let need_bytes = serde_json::to_vec(need).expect("Failed to serialize need");

        // Evaluate
        let result = evaluate(&policy_bytes, &snapshot_bytes, &need_bytes, evaluated_at);

        // Check result against expected
        if let Some(_expected_error) = test_case.get("expectedError") {
            // Error case
            match result {
                Ok(_) => {
                    println!("✗ Case {}: {} - Expected error but got success", i, case_id);
                    failed += 1;
                }
                Err(_) => {
                    println!("✓ Case {}: {} (error case passed)", i, case_id);
                    passed += 1;
                }
            }
        } else if let Some(_expected_evaluation) = test_case.get("expectedEvaluation") {
            // Success case
            match result {
                Err(e) => {
                    println!(
                        "✗ Case {}: {} - Expected success but got error: {:?}",
                        i, case_id, e
                    );
                    failed += 1;
                }
                Ok(evaluation_jcs) => {
                    // Parse result as JSON
                    let result_json: serde_json::Value = serde_json::from_slice(&evaluation_jcs)
                        .expect("Failed to parse result as JSON");

                    // Full conformance (SEMANTICS §10): the engine's evaluation must match
                    // expectedEvaluation EXACTLY on every field — verdict, id, tenantId,
                    // policyId, policyDigest, snapshotId, snapshotDigest, needDigest, and
                    // per-rule reason codes. Structural (order-independent) JSON equality.
                    let expected_evaluation = test_case
                        .get("expectedEvaluation")
                        .expect("expectedEvaluation present");
                    if result_json == *expected_evaluation {
                        println!("✓ Case {}: {} (passed — full field match)", i, case_id);
                        passed += 1;
                    } else {
                        println!(
                            "✗ Case {}: {} - evaluation mismatch:\n  got:      {}\n  expected: {}",
                            i, case_id, result_json, expected_evaluation
                        );
                        failed += 1;
                    }
                }
            }
        } else {
            panic!(
                "Case {}: {} - No expectedError or expectedEvaluation in golden.json",
                i, case_id
            );
        }
    }

    println!(
        "\nResults: {} passed, {} failed out of {} total",
        passed,
        failed,
        cases.len()
    );
    assert_eq!(
        failed, 0,
        "Some conformance tests failed. See output above for details."
    );
}
