//! Host-side checks: the local engine path end to end on the demo data, and
//! an SSR smoke render of the shell.

use dioxus::prelude::*;
use rumble_ai_clearance_domain::Hosting;
use rumble_ai_clearance_web::{
    App, DEMO_SNAPSHOT, EXAMPLE_ORG_POLICY, evaluate_locally, explain_locally,
    normalize_server_base_url, parse_remote_outcome,
};

#[test]
fn demo_data_evaluates_under_the_example_policy() {
    let outcome = evaluate_locally(
        DEMO_SNAPSHOT,
        EXAMPLE_ORG_POLICY,
        "code_generation",
        "public_content",
        "c0",
    )
    .expect("demo data evaluates");

    let ids: Vec<&str> = outcome
        .eligible
        .iter()
        .map(|row| row.model.as_str())
        .collect();
    // Self-hostable models survive "no US/CN data flow"; API-only US models
    // do not.
    assert!(ids.contains(&"mistralai/mistral-large"), "got: {ids:?}");
    assert!(ids.contains(&"meta/llama-3-3-70b"));
    assert!(!ids.contains(&"openai/gpt-4o"));
    assert!(outcome.ineligible_count >= 2);
}

#[test]
fn sensitive_pii_need_tightens_the_demo_verdicts() {
    let public = evaluate_locally(
        DEMO_SNAPSHOT,
        EXAMPLE_ORG_POLICY,
        "summary_extraction",
        "public_content",
        "c0",
    )
    .expect("evaluates");
    let pii_c2 = evaluate_locally(
        DEMO_SNAPSHOT,
        EXAMPLE_ORG_POLICY,
        "summary_extraction",
        "personal_data",
        "c2",
    )
    .expect("evaluates");

    // Monotonicity, visible end to end: stricter need, never more models.
    assert!(pii_c2.eligible.len() <= public.eligible.len());
}

#[test]
fn unknown_model_explains_as_denied() {
    let lines = explain_locally(
        DEMO_SNAPSHOT,
        EXAMPLE_ORG_POLICY,
        "general_chat",
        "public_content",
        "c0",
        "acme/never-heard-of",
    )
    .expect("explains");

    let text = lines.join("\n");
    assert!(text.contains("INELIGIBLE"), "got: {text}");
    assert!(text.contains("builtin.unknown-model"));
}

#[test]
fn server_base_url_is_required_and_normalized() {
    assert_eq!(
        normalize_server_base_url(" https://clearance.example/ ").unwrap(),
        "https://clearance.example"
    );
    assert!(normalize_server_base_url("  ").is_err());
}

#[test]
fn remote_contract_preserves_viable_hostings() {
    let payload = serde_json::json!({
        "data": {
            "eligible": [{
                "model": "mistralai/mistral-large",
                "viable_hostings": [Hosting::SelfHosted]
            }],
            "ineligible_count": 2,
            "indeterminate_count": 1
        },
        "meta": { "snapshot_generated_at": "2026-07-10T00:00:00Z" }
    });

    let outcome = parse_remote_outcome(&payload).expect("valid API contract");
    assert_eq!(outcome.ineligible_count, 2);
    assert_eq!(outcome.indeterminate_count, 1);
    assert_eq!(outcome.eligible[0].hostings, ["self-hosted"]);
}

#[test]
fn malformed_remote_counts_fail_closed() {
    let payload = serde_json::json!({
        "data": {
            "eligible": [],
            "ineligible_count": "2",
            "indeterminate_count": 1
        },
        "meta": { "snapshot_generated_at": "2026-07-10T00:00:00Z" }
    });

    assert!(parse_remote_outcome(&payload).is_err());
}

#[test]
fn shell_renders_with_attribution_and_form() {
    let mut dom = VirtualDom::new(App);
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);

    assert!(html.contains("rumble-ai-clearance"));
    assert!(html.contains("Artificial Analysis"));
    assert!(html.contains("Evaluate"));
    assert!(html.contains("code_generation"));
}
