//! The shipped rulebook must always parse and compile: fail-closed applies
//! to our own content first.

use rumble_ai_clearance_domain::{
    BenchDimension, CountryCode, Hosting, Model, NeedProfile, Purpose, Sensitivity, Task, Verdict,
    evaluate,
};
use rumble_ai_clearance_policy::{compile, compile_ranking, parse_policy};

const SHIPPED_RULEBOOK: &str = include_str!("../../../content/rulebook/rulebook.yaml");

#[test]
fn shipped_rulebook_parses_and_compiles() {
    let rulebook = parse_policy(SHIPPED_RULEBOOK).expect("shipped rulebook parses");
    let empty_org = parse_policy("version: 1\nrules: []\n").expect("empty org");

    compile(&rulebook, &empty_org).expect("shipped rulebook compiles alone");
}

#[test]
fn shipped_rulebook_keeps_c2_data_in_eu_or_self_hosted() {
    let rulebook = parse_policy(SHIPPED_RULEBOOK).expect("shipped rulebook parses");
    let empty_org = parse_policy("version: 1\nrules: []\n").expect("empty org");
    let policy = compile(&rulebook, &empty_org).expect("compiles");

    // The canonical brief: US-origin model, self-hosted → fine even at C2.
    let self_hosted_us = Model::new("meta/llama-4", "meta")
        .with_origin(CountryCode::new("US"))
        .with_hosting(Hosting::SelfHosted);
    let c2_need = NeedProfile::new(
        Task::SummaryExtraction,
        Purpose::PublicContent,
        Sensitivity::C2,
    );
    assert!(matches!(
        evaluate(&self_hosted_us, &policy, &c2_need),
        Verdict::Eligible { .. }
    ));

    // Same model reachable only through a US-jurisdiction API → out at C2.
    let api_us = Model::new("openai/gpt-6", "openai")
        .with_origin(CountryCode::new("US"))
        .with_hosting(Hosting::api(
            rumble_ai_clearance_domain::ApiKind::Provider,
            CountryCode::new("US"),
            CountryCode::new("US"),
        ));
    assert!(matches!(
        evaluate(&api_us, &policy, &c2_need),
        Verdict::Ineligible { .. }
    ));

    // And at C0 the same API path is fine: the rule is sensitivity-scoped.
    let c0_need = NeedProfile::new(
        Task::SummaryExtraction,
        Purpose::PublicContent,
        Sensitivity::C0,
    );
    assert!(matches!(
        evaluate(&api_us, &policy, &c0_need),
        Verdict::Eligible { .. }
    ));
}

#[test]
fn shipped_ranking_covers_every_task() {
    let rulebook = parse_policy(SHIPPED_RULEBOOK).expect("shipped rulebook parses");
    let empty_org = parse_policy("version: 1\nrules: []\n").expect("empty org");
    let ranking = compile_ranking(&rulebook, &empty_org);

    // Spot-check two mappings from the shipped defaults.
    assert_eq!(
        ranking.spec_for(Task::CodeGeneration),
        rumble_ai_clearance_domain::RankingSpec::new(vec![
            BenchDimension::Coding,
            BenchDimension::Intelligence,
        ])
    );
    assert_eq!(
        ranking.spec_for(Task::Translation),
        rumble_ai_clearance_domain::RankingSpec::new(vec![
            BenchDimension::Multilingual,
            BenchDimension::Intelligence,
        ])
    );
}
