//! Connectors and merge, exercised on recorded fixtures — never live HTTP.

use rumble_ai_clearance_dataset::parse_governance;
use rumble_ai_clearance_domain::{
    BenchDimension, Constraint, CountryCode, NeedProfile, Openness, Policy, Purpose, Rule,
    Sensitivity, Task, Verdict, evaluate,
};
use rumble_ai_clearance_sync::{
    SyncTimestamps, build_snapshot, parse_aa_response, parse_hf_response,
};

const AA_FIXTURE: &str = include_str!("fixtures/aa-models.json");
const HF_FIXTURE: &str = include_str!("fixtures/hf-models.json");
const SHIPPED_PROVIDERS: &str = include_str!("../../../content/governance/providers.yaml");

fn timestamps() -> SyncTimestamps {
    SyncTimestamps {
        generated_at: "2026-07-10T12:00:00Z".to_string(),
        aa_fetched_at: Some("2026-07-10T11:58:00Z".to_string()),
        hf_fetched_at: Some("2026-07-10T11:59:00Z".to_string()),
        curated_version: "curated-v1".to_string(),
    }
}

#[test]
fn aa_response_parses_models_indices_and_prices() {
    let models = parse_aa_response(AA_FIXTURE).expect("fixture parses");

    assert_eq!(models.len(), 3);
    let mistral = &models[0];
    assert_eq!(mistral.slug(), "mistral-large-3");
    assert_eq!(mistral.creator_name(), "Mistral AI");
    assert_eq!(mistral.index(BenchDimension::Coding), Some(39.1));
    assert_eq!(mistral.index(BenchDimension::Math), Some(52.3));
    assert_eq!(mistral.price_input(), Some(2.0));

    // Math index absent on GPT-6 in the fixture: stays unknown, never zero.
    assert_eq!(models[1].index(BenchDimension::Math), None);
}

#[test]
fn hf_response_parses_licences_and_gating() {
    let models = parse_hf_response(HF_FIXTURE).expect("fixture parses");

    assert_eq!(models.len(), 3);
    assert_eq!(models[0].repo_id(), "mistralai/Mistral-Large-3");
    assert_eq!(models[0].license(), Some("apache-2.0"));
    assert_eq!(models[1].license(), Some("llama4"));
    assert_eq!(models[2].license(), None);
}

#[test]
fn merged_snapshot_takes_origin_from_curated_governance() {
    let governance = parse_governance(SHIPPED_PROVIDERS).expect("governance parses");
    let aa = parse_aa_response(AA_FIXTURE).expect("aa parses");
    let hf = parse_hf_response(HF_FIXTURE).expect("hf parses");

    let snapshot = build_snapshot(&governance, &aa, &hf, timestamps()).expect("builds");
    let models = snapshot.models();

    // AA + HF matched: Mistral gets curated FR origin, a self-host path from
    // HF weights, and its API path under FR jurisdiction (provider HQ).
    let mistral = models
        .iter()
        .find(|m| m.id() == "mistralai/mistral-large-3")
        .expect("mistral present");
    let strict_eu = Policy::new(vec![Rule::always(
        "org.eu-or-selfhost",
        Constraint::RequireHostingJurisdictionIn(vec![CountryCode::new("FR")]),
    )]);
    let need = NeedProfile::new(Task::GeneralChat, Purpose::PublicContent, Sensitivity::C0);
    assert!(matches!(
        evaluate(mistral, &strict_eu, &need),
        Verdict::Eligible { .. }
    ));

    // Closed model: no self-host path, US jurisdiction API only.
    let gpt6 = models
        .iter()
        .find(|m| m.id() == "openai/gpt-6")
        .expect("gpt-6 present");
    assert!(matches!(
        evaluate(gpt6, &strict_eu, &need),
        Verdict::Ineligible { .. }
    ));

    // Unknown creator: kept, but with no curated origin — an origin rule
    // must leave it Indeterminate (deny-by-default), never eligible.
    let frontier = models
        .iter()
        .find(|m| m.id().ends_with("/frontier-x"))
        .expect("frontier present");
    let origin_rule = Policy::new(vec![Rule::always(
        "org.deny-origin-us",
        Constraint::DenyOrigin(vec![CountryCode::new("US")]),
    )]);
    assert!(matches!(
        evaluate(frontier, &origin_rule, &need),
        Verdict::Indeterminate { .. }
    ));
}

#[test]
fn licences_classify_openness_fail_closed() {
    let governance = parse_governance(SHIPPED_PROVIDERS).expect("governance parses");
    let aa = parse_aa_response(AA_FIXTURE).expect("aa parses");
    let hf = parse_hf_response(HF_FIXTURE).expect("hf parses");

    let snapshot = build_snapshot(&governance, &aa, &hf, timestamps()).expect("builds");
    let models = snapshot.models();
    let need = NeedProfile::new(Task::GeneralChat, Purpose::PublicContent, Sensitivity::C0);

    let open_source_only = Policy::new(vec![Rule::always(
        "org.open-source-only",
        Constraint::RequireOpenness(vec![Openness::OpenSource]),
    )]);

    // apache-2.0 → open_source.
    let mistral = models
        .iter()
        .find(|m| m.id() == "mistralai/mistral-large-3")
        .expect("mistral present");
    assert!(matches!(
        evaluate(mistral, &open_source_only, &need),
        Verdict::Eligible { .. }
    ));

    // llama4 licence → open_weight, not open_source.
    let llama = models
        .iter()
        .find(|m| m.id() == "meta/llama-4")
        .expect("llama present");
    assert!(matches!(
        evaluate(llama, &open_source_only, &need),
        Verdict::Ineligible { .. }
    ));

    // No licence tag → openness unknown → Indeterminate, never eligible.
    let unlicensed = models
        .iter()
        .find(|m| m.id().ends_with("/no-license-model"))
        .expect("unlicensed present");
    assert!(matches!(
        evaluate(unlicensed, &open_source_only, &need),
        Verdict::Indeterminate { .. }
    ));
}

#[test]
fn manifest_cites_all_three_sources() {
    let governance = parse_governance(SHIPPED_PROVIDERS).expect("governance parses");
    let aa = parse_aa_response(AA_FIXTURE).expect("aa parses");
    let hf = parse_hf_response(HF_FIXTURE).expect("hf parses");

    let snapshot = build_snapshot(&governance, &aa, &hf, timestamps()).expect("builds");

    assert_eq!(snapshot.manifest().generated_at(), "2026-07-10T12:00:00Z");
    assert_eq!(snapshot.manifest().sources().len(), 3);
}
