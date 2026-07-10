//! Policy documents: parsing, rulebook ⊕ org merge, fail-closed validation.

use rumble_ai_clearance_domain::{
    BenchDimension, CountryCode, Model, NeedProfile, Purpose, RankingSpec, RuleId, Sensitivity,
    Task, Verdict, evaluate,
};
use rumble_ai_clearance_policy::{PolicyError, compile, compile_ranking, parse_need, parse_policy};

const RULEBOOK: &str = r#"
version: 1
rules:
  - id: rulebook.sensitive-data-deny-us-jurisdiction
    description: Sensitive data must not flow under US jurisdiction
    source: "CLOUD Act, 18 U.S.C. 2713"
    applies:
      sensitivity_at_least: c2
    constraint:
      deny_hosting_jurisdiction: [US]
"#;

const ORG_POLICY: &str = r#"
version: 1
organization: ACME
rules:
  - id: org.deny-origin-us-cn
    description: No US or Chinese model artifacts
    applies: always
    constraint:
      deny_origin: [US, CN]
"#;

#[test]
fn compiles_rulebook_and_org_rules_into_effective_policy() {
    let rulebook = parse_policy(RULEBOOK).expect("rulebook parses");
    let org = parse_policy(ORG_POLICY).expect("org policy parses");

    let policy = compile(&rulebook, &org).expect("compiles");

    let model = Model::new("deepseek/deepseek-v4", "deepseek")
        .with_origin(CountryCode::new("CN"))
        .with_hosting(rumble_ai_clearance_domain::Hosting::SelfHosted);
    let need = NeedProfile::new(Task::GeneralChat, Purpose::PublicContent, Sensitivity::C0);

    assert_eq!(
        evaluate(&model, &policy, &need),
        Verdict::Ineligible {
            violations: vec![RuleId::new("org.deny-origin-us-cn")],
        }
    );
}

#[test]
fn org_can_disable_a_rulebook_rule_with_a_reason() {
    let org_yaml = r#"
version: 1
organization: ACME
disable_rules:
  - id: rulebook.sensitive-data-deny-us-jurisdiction
    reason: UK adequacy decision accepted by legal, 2026-03
rules: []
"#;
    let rulebook = parse_policy(RULEBOOK).expect("rulebook parses");
    let org = parse_policy(org_yaml).expect("org policy parses");

    let policy = compile(&rulebook, &org).expect("compiles");

    // The C2 rule is gone: a US-jurisdiction API path stays viable even at C2.
    let model = Model::new("openai/gpt-6", "openai")
        .with_origin(CountryCode::new("US"))
        .with_hosting(rumble_ai_clearance_domain::Hosting::api(
            rumble_ai_clearance_domain::ApiKind::Provider,
            CountryCode::new("US"),
            CountryCode::new("US"),
        ));
    let need = NeedProfile::new(Task::GeneralChat, Purpose::PublicContent, Sensitivity::C2);

    assert!(matches!(
        evaluate(&model, &policy, &need),
        Verdict::Eligible { .. }
    ));
}

#[test]
fn disabling_an_unknown_rule_is_an_error() {
    let org_yaml = r#"
version: 1
disable_rules:
  - id: rulebook.does-not-exist
    reason: typo
rules: []
"#;
    let rulebook = parse_policy(RULEBOOK).expect("rulebook parses");
    let org = parse_policy(org_yaml).expect("org policy parses");

    assert!(matches!(
        compile(&rulebook, &org),
        Err(PolicyError::UnknownDisabledRule(id)) if id == "rulebook.does-not-exist"
    ));
}

#[test]
fn disabling_without_a_reason_fails_to_parse() {
    let org_yaml = r#"
version: 1
disable_rules:
  - id: rulebook.sensitive-data-deny-us-jurisdiction
rules: []
"#;
    assert!(matches!(
        parse_policy(org_yaml),
        Err(PolicyError::Yaml { .. })
    ));
}

#[test]
fn duplicate_rule_ids_are_an_error() {
    let org_yaml = r#"
version: 1
rules:
  - id: rulebook.sensitive-data-deny-us-jurisdiction
    applies: always
    constraint:
      deny_origin: [US]
"#;
    let rulebook = parse_policy(RULEBOOK).expect("rulebook parses");
    let org = parse_policy(org_yaml).expect("org policy parses");

    assert!(matches!(
        compile(&rulebook, &org),
        Err(PolicyError::DuplicateRuleId(id))
            if id == "rulebook.sensitive-data-deny-us-jurisdiction"
    ));
}

#[test]
fn unknown_fields_fail_to_parse() {
    let org_yaml = r#"
version: 1
rules:
  - id: org.rule
    applies: always
    constraint:
      deny_origin: [US]
    severity: high
"#;
    assert!(matches!(
        parse_policy(org_yaml),
        Err(PolicyError::Yaml { .. })
    ));
}

#[test]
fn every_constraint_kind_round_trips_from_yaml() {
    let org_yaml = r#"
version: 1
rules:
  - id: org.a
    applies: always
    constraint:
      deny_origin: [US]
  - id: org.b
    applies: always
    constraint:
      deny_hosting_jurisdiction: [US, CN]
  - id: org.c
    applies: always
    constraint:
      deny_provider: [acme]
  - id: org.d
    applies: always
    constraint:
      require_openness: [open_weight, open_source]
  - id: org.e
    applies: always
    constraint:
      require_self_hostable: true
  - id: org.f
    applies:
      task: code_generation
    constraint:
      min_context_window: 128000
  - id: org.g
    applies:
      purpose: health_data
    constraint:
      min_bench:
        dimension: coding
        threshold: 40.5
"#;
    let empty_rulebook = parse_policy("version: 1\nrules: []\n").expect("empty rulebook");
    let org = parse_policy(org_yaml).expect("org policy parses");

    let policy = compile(&empty_rulebook, &org).expect("compiles");

    // Spot-check through evaluation: the require_self_hostable rule bites.
    let model = Model::new("acme2/closed", "acme2")
        .with_origin(CountryCode::new("FR"))
        .with_openness(rumble_ai_clearance_domain::Openness::OpenWeight)
        .with_self_hostable(false)
        .with_hosting(rumble_ai_clearance_domain::Hosting::SelfHosted);
    let need = NeedProfile::new(Task::GeneralChat, Purpose::PublicContent, Sensitivity::C0);

    assert_eq!(
        evaluate(&model, &policy, &need),
        Verdict::Ineligible {
            violations: vec![RuleId::new("org.e")],
        }
    );
}

#[test]
fn jurisdiction_allow_list_parses_and_applies() {
    let org_yaml = r#"
version: 1
rules:
  - id: org.eu-or-selfhost
    applies:
      sensitivity_at_least: c2
    constraint:
      require_hosting_jurisdiction_in: [FR, DE]
"#;
    let empty_rulebook = parse_policy("version: 1\nrules: []\n").expect("empty rulebook");
    let org = parse_policy(org_yaml).expect("org policy parses");
    let policy = compile(&empty_rulebook, &org).expect("compiles");

    let us_api_only = Model::new("openai/gpt-6", "openai")
        .with_origin(CountryCode::new("US"))
        .with_hosting(rumble_ai_clearance_domain::Hosting::api(
            rumble_ai_clearance_domain::ApiKind::Provider,
            CountryCode::new("US"),
            CountryCode::new("US"),
        ));
    let need = NeedProfile::new(Task::GeneralChat, Purpose::PublicContent, Sensitivity::C2);

    assert_eq!(
        evaluate(&us_api_only, &policy, &need),
        Verdict::Ineligible {
            violations: vec![RuleId::new("org.eu-or-selfhost")],
        }
    );
}

#[test]
fn ranking_section_maps_tasks_to_bench_dimensions() {
    let rulebook_yaml = r#"
version: 1
ranking:
  code_generation: [coding, intelligence]
  general_chat: [intelligence]
rules: []
"#;
    let org_yaml = r#"
version: 1
ranking:
  general_chat: [multilingual]
rules: []
"#;
    let rulebook = parse_policy(rulebook_yaml).expect("rulebook parses");
    let org = parse_policy(org_yaml).expect("org policy parses");

    let ranking = compile_ranking(&rulebook, &org);

    // Rulebook default kept where the org says nothing…
    assert_eq!(
        ranking.spec_for(Task::CodeGeneration),
        RankingSpec::new(vec![BenchDimension::Coding, BenchDimension::Intelligence])
    );
    // …org override wins where it speaks.
    assert_eq!(
        ranking.spec_for(Task::GeneralChat),
        RankingSpec::new(vec![BenchDimension::Multilingual])
    );
    // Unmapped tasks fall back to Intelligence.
    assert_eq!(
        ranking.spec_for(Task::Writing),
        RankingSpec::new(vec![BenchDimension::Intelligence])
    );
}

#[test]
fn require_self_hostable_false_is_rejected() {
    let org_yaml = r#"
version: 1
rules:
  - id: org.weird
    applies: always
    constraint:
      require_self_hostable: false
"#;
    let empty_rulebook = parse_policy("version: 1\nrules: []\n").expect("empty rulebook");
    let org = parse_policy(org_yaml).expect("org policy parses");

    assert!(matches!(
        compile(&empty_rulebook, &org),
        Err(PolicyError::InvalidRule { .. })
    ));
}

#[test]
fn need_profile_parses_from_yaml() {
    let need = parse_need("task: summary_extraction\npurpose: personal_data\nsensitivity: c2\n")
        .expect("need parses");

    assert_eq!(
        need,
        NeedProfile::new(
            Task::SummaryExtraction,
            Purpose::PersonalData,
            Sensitivity::C2
        )
    );

    assert!(parse_need("task: nonsense\npurpose: public_content\nsensitivity: c0\n").is_err());
}

#[test]
fn empty_ranking_dimension_list_is_rejected() {
    // An empty list would silently degrade ranking to price-only: refuse it.
    let yaml = r#"
version: 1
ranking:
  code_generation: []
rules: []
"#;
    assert!(parse_policy(yaml).is_err());
}
