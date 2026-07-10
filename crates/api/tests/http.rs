//! API contracts, endpoint by endpoint. Read-only by construction: the
//! router exposes no mutating route at all.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use rumble_ai_clearance_api::{AppState, build_router};
use rumble_ai_clearance_dataset::parse_governance;
use rumble_ai_clearance_policy::{compile, compile_ranking, parse_policy};
use rumble_ai_clearance_sync::{
    SyncTimestamps, build_snapshot, parse_aa_response, parse_hf_response,
};

const AA_FIXTURE: &str = include_str!("../../sync/tests/fixtures/aa-models.json");
const HF_FIXTURE: &str = include_str!("../../sync/tests/fixtures/hf-models.json");
const SHIPPED_PROVIDERS: &str = include_str!("../../../content/governance/providers.yaml");
const SHIPPED_RULEBOOK: &str = include_str!("../../../content/rulebook/rulebook.yaml");
const EXAMPLE_POLICY: &str = include_str!("../../../examples/policy-no-us-cn-selfhost-ok.yaml");

fn state() -> AppState {
    let governance = parse_governance(SHIPPED_PROVIDERS).expect("governance");
    let aa = parse_aa_response(AA_FIXTURE).expect("aa");
    let hf = parse_hf_response(HF_FIXTURE).expect("hf");
    let snapshot = build_snapshot(
        &governance,
        &aa,
        &hf,
        SyncTimestamps {
            generated_at: "2026-07-10T12:00:00Z".to_string(),
            aa_fetched_at: Some("2026-07-10T11:58:00Z".to_string()),
            hf_fetched_at: Some("2026-07-10T11:59:00Z".to_string()),
            curated_version: "curated-v1".to_string(),
        },
    )
    .expect("snapshot");

    let rulebook = parse_policy(SHIPPED_RULEBOOK).expect("rulebook");
    let org = parse_policy(EXAMPLE_POLICY).expect("org policy");
    let policy = compile(&rulebook, &org).expect("effective policy");
    let ranking = compile_ranking(&rulebook, &org);

    AppState::new(snapshot, policy, ranking)
}

async fn get_json(uri: &str) -> (StatusCode, serde_json::Value) {
    let response = build_router(state())
        .oneshot(Request::get(uri).body(Body::empty()).expect("request"))
        .await
        .expect("response");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let value = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("json body")
    };
    (status, value)
}

#[tokio::test]
async fn dataset_endpoint_cites_the_snapshot_version() {
    let (status, body) = get_json("/api/v1/dataset").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["generated_at"], "2026-07-10T12:00:00Z");
    assert_eq!(
        body["data"]["sources"].as_array().expect("sources").len(),
        3
    );
}

#[tokio::test]
async fn models_endpoint_paginates_with_cursor() {
    let (status, first) = get_json("/api/v1/models?limit=2").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first["data"].as_array().expect("data").len(), 2);
    let cursor = first["meta"]["next_cursor"].as_str().expect("next cursor");

    let (status, second) = get_json(&format!("/api/v1/models?limit=50&cursor={cursor}")).await;
    assert_eq!(status, StatusCode::OK);
    let rest = second["data"].as_array().expect("data");
    assert_eq!(rest.len(), 3, "5 models total, 2 already served");
    assert!(second["meta"]["next_cursor"].is_null());

    // Ids are strictly increasing across pages: deterministic pagination.
    let first_last = first["data"][1]["id"].as_str().expect("id");
    let second_first = rest[0]["id"].as_str().expect("id");
    assert!(first_last < second_first);
}

#[tokio::test]
async fn evaluations_endpoint_ranks_eligible_models() {
    let body = serde_json::json!({
        "task": "code_generation",
        "purpose": "public_content",
        "sensitivity": "c0",
    });
    let response = build_router(state())
        .oneshot(
            Request::post("/api/v1/evaluations")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let report: serde_json::Value = serde_json::from_slice(&bytes).expect("json");

    let eligible = report["data"]["eligible"].as_array().expect("eligible");
    assert_eq!(eligible[0]["model"], "mistralai/mistral-large-3");
    assert_eq!(report["data"]["ineligible_count"], 1);
    assert_eq!(report["data"]["indeterminate_count"], 1);
    assert_eq!(
        report["meta"]["snapshot_generated_at"],
        "2026-07-10T12:00:00Z"
    );
}

#[tokio::test]
async fn evaluations_endpoint_rejects_malformed_needs() {
    let response = build_router(state())
        .oneshot(
            Request::post("/api/v1/evaluations")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"task":"nonsense"}"#))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn verdicts_endpoint_explains_and_denies_unknown_models() {
    let (status, known) = get_json(
        "/api/v1/verdicts?model=openai/gpt-6&task=code_generation&purpose=public_content&sensitivity=c0",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(known["data"]["model"], "openai/gpt-6");
    let verdict = known["data"]["verdict"].to_string();
    assert!(verdict.contains("ineligible"), "got: {verdict}");

    let (status, unknown) = get_json(
        "/api/v1/verdicts?model=acme/never-heard-of&task=code_generation&purpose=public_content&sensitivity=c0",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        unknown["data"]["verdict"]
            .to_string()
            .contains("builtin.unknown-model"),
        "unknown model must be denied, got: {unknown}"
    );
}

#[tokio::test]
async fn policy_endpoint_lists_effective_rules() {
    let (status, body) = get_json("/api/v1/policy").await;

    assert_eq!(status, StatusCode::OK);
    let text = body["data"].to_string();
    assert!(text.contains("org.no-us-cn-data-flow"), "got: {text}");
    assert!(text.contains("rulebook.c2-eu-jurisdiction-or-selfhost"));
}
