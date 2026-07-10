//! Eligibility engine behavior, exercised through the public API only.

use rumble_ai_clearance_domain::{
    ApiKind, BenchDimension, Constraint, CountryCode, DataDimension, Hosting, Model, NeedProfile,
    Openness, Policy, Price, ProviderId, Purpose, RankingSpec, Rule, RuleId, Sensitivity, Task,
    Verdict, evaluate, rank, verdict_for,
};

fn any_need() -> NeedProfile {
    NeedProfile::new(Task::GeneralChat, Purpose::PublicContent, Sensitivity::C0)
}

#[test]
fn jurisdiction_ban_leaves_only_untainted_hostings() {
    // "No data flow to US/CN, but self-hosting a US-origin model is fine":
    // the ban targets hosting jurisdiction, not model origin.
    let model = Model::new("meta/llama-4", "meta")
        .with_origin(CountryCode::new("US"))
        .with_hosting(Hosting::SelfHosted)
        .with_hosting(Hosting::api(
            ApiKind::Provider,
            CountryCode::new("US"),
            CountryCode::new("US"),
        ));
    let policy = Policy::new(vec![Rule::always(
        "org.deny-jurisdiction-us-cn",
        Constraint::DenyHostingJurisdiction(vec![CountryCode::new("US"), CountryCode::new("CN")]),
    )]);

    let verdict = evaluate(&model, &policy, &any_need());

    assert_eq!(
        verdict,
        Verdict::Eligible {
            viable_hostings: vec![Hosting::SelfHosted],
        }
    );
}

#[test]
fn jurisdiction_ban_with_no_compliant_path_is_ineligible() {
    let model = Model::new("openai/gpt-6", "openai")
        .with_origin(CountryCode::new("US"))
        .with_hosting(Hosting::api(
            ApiKind::Provider,
            CountryCode::new("US"),
            CountryCode::new("US"),
        ))
        .with_hosting(Hosting::api(
            ApiKind::Hyperscaler,
            CountryCode::new("IE"),
            // Hosted in the EU but answering to US law: still banned.
            CountryCode::new("US"),
        ));
    let policy = Policy::new(vec![Rule::always(
        "org.deny-jurisdiction-us",
        Constraint::DenyHostingJurisdiction(vec![CountryCode::new("US")]),
    )]);

    let verdict = evaluate(&model, &policy, &any_need());

    assert_eq!(
        verdict,
        Verdict::Ineligible {
            violations: vec![RuleId::new("org.deny-jurisdiction-us")],
        }
    );
}

#[test]
fn unknown_jurisdiction_on_only_path_is_indeterminate() {
    let model = Model::new("acme/opaque-model", "acme")
        .with_origin(CountryCode::new("FR"))
        .with_hosting(Hosting::Api {
            kind: ApiKind::Provider,
            country: Some(CountryCode::new("FR")),
            jurisdiction: None,
        });
    let policy = Policy::new(vec![Rule::always(
        "org.deny-jurisdiction-us",
        Constraint::DenyHostingJurisdiction(vec![CountryCode::new("US")]),
    )]);

    let verdict = evaluate(&model, &policy, &any_need());

    assert_eq!(
        verdict,
        Verdict::Indeterminate {
            missing: vec![(
                RuleId::new("org.deny-jurisdiction-us"),
                DataDimension::HostingJurisdiction
            )],
        }
    );
}

#[test]
fn jurisdiction_allow_list_keeps_allowed_and_self_hosted_paths() {
    // "C2 data: EU jurisdiction or self-host" — allow-list semantics.
    let model = Model::new("mistralai/mistral-large-3", "mistralai")
        .with_origin(CountryCode::new("FR"))
        .with_hosting(Hosting::SelfHosted)
        .with_hosting(Hosting::api(
            ApiKind::EuSovereign,
            CountryCode::new("FR"),
            CountryCode::new("FR"),
        ))
        .with_hosting(Hosting::api(
            ApiKind::Hyperscaler,
            CountryCode::new("IE"),
            CountryCode::new("US"),
        ));
    let policy = Policy::new(vec![Rule::always(
        "rulebook.eu-or-selfhost",
        Constraint::RequireHostingJurisdictionIn(vec![
            CountryCode::new("FR"),
            CountryCode::new("DE"),
        ]),
    )]);

    let verdict = evaluate(&model, &policy, &any_need());

    assert_eq!(
        verdict,
        Verdict::Eligible {
            viable_hostings: vec![
                Hosting::SelfHosted,
                Hosting::api(
                    ApiKind::EuSovereign,
                    CountryCode::new("FR"),
                    CountryCode::new("FR"),
                ),
            ],
        }
    );
}

#[test]
fn empty_jurisdiction_allow_list_means_self_host_only() {
    let policy = Policy::new(vec![Rule::always(
        "rulebook.self-host-only",
        Constraint::RequireHostingJurisdictionIn(vec![]),
    )]);

    let self_hostable = Model::new("meta/llama-4", "meta")
        .with_origin(CountryCode::new("US"))
        .with_hosting(Hosting::SelfHosted)
        .with_hosting(Hosting::api(
            ApiKind::Provider,
            CountryCode::new("US"),
            CountryCode::new("US"),
        ));
    assert_eq!(
        evaluate(&self_hostable, &policy, &any_need()),
        Verdict::Eligible {
            viable_hostings: vec![Hosting::SelfHosted],
        }
    );

    let api_only = Model::new("openai/gpt-6", "openai")
        .with_origin(CountryCode::new("US"))
        .with_hosting(Hosting::api(
            ApiKind::Provider,
            CountryCode::new("US"),
            CountryCode::new("US"),
        ));
    assert_eq!(
        evaluate(&api_only, &policy, &any_need()),
        Verdict::Ineligible {
            violations: vec![RuleId::new("rulebook.self-host-only")],
        }
    );
}

#[test]
fn model_without_any_documented_hosting_path_is_indeterminate() {
    // Deny-by-default: no documented way to run the model is missing data,
    // never eligibility.
    let model = Model::new("acme/paper-model", "acme").with_origin(CountryCode::new("FR"));
    let policy = Policy::new(vec![]);

    let verdict = evaluate(&model, &policy, &any_need());

    assert_eq!(
        verdict,
        Verdict::Indeterminate {
            missing: vec![(RuleId::builtin_hosting_paths(), DataDimension::HostingPaths)],
        }
    );
}

#[test]
fn sensitivity_scoped_rule_activates_at_its_threshold_only() {
    let model = Model::new("openai/gpt-6", "openai")
        .with_origin(CountryCode::new("US"))
        .with_hosting(Hosting::api(
            ApiKind::Provider,
            CountryCode::new("US"),
            CountryCode::new("US"),
        ));
    let policy = Policy::new(vec![Rule::when_sensitivity_at_least(
        Sensitivity::C2,
        "rulebook.sensitive-data-deny-us-jurisdiction",
        Constraint::DenyHostingJurisdiction(vec![CountryCode::new("US")]),
    )]);

    let public_need = NeedProfile::new(Task::GeneralChat, Purpose::PublicContent, Sensitivity::C0);
    assert!(matches!(
        evaluate(&model, &policy, &public_need),
        Verdict::Eligible { .. }
    ));

    let sensitive_need =
        NeedProfile::new(Task::GeneralChat, Purpose::PublicContent, Sensitivity::C2);
    assert_eq!(
        evaluate(&model, &policy, &sensitive_need),
        Verdict::Ineligible {
            violations: vec![RuleId::new("rulebook.sensitive-data-deny-us-jurisdiction")],
        }
    );
}

fn fr_self_hosted_model(id: &str, provider: &str) -> Model {
    Model::new(id, provider)
        .with_origin(CountryCode::new("FR"))
        .with_hosting(Hosting::SelfHosted)
}

#[test]
fn require_self_hostable_bans_api_only_model() {
    let model =
        fr_self_hosted_model("mistralai/mistral-large-3", "mistralai").with_self_hostable(false);
    let policy = Policy::new(vec![Rule::always(
        "org.require-self-hostable",
        Constraint::RequireSelfHostable,
    )]);

    assert_eq!(
        evaluate(&model, &policy, &any_need()),
        Verdict::Ineligible {
            violations: vec![RuleId::new("org.require-self-hostable")],
        }
    );
}

#[test]
fn require_openness_bans_closed_model_and_unknown_is_indeterminate() {
    let policy = Policy::new(vec![Rule::always(
        "org.require-open-weights",
        Constraint::RequireOpenness(vec![Openness::OpenWeight, Openness::OpenSource]),
    )]);

    let closed = fr_self_hosted_model("acme/closed-model", "acme").with_openness(Openness::Closed);
    assert_eq!(
        evaluate(&closed, &policy, &any_need()),
        Verdict::Ineligible {
            violations: vec![RuleId::new("org.require-open-weights")],
        }
    );

    let unknown = fr_self_hosted_model("acme/mystery-model", "acme");
    assert_eq!(
        evaluate(&unknown, &policy, &any_need()),
        Verdict::Indeterminate {
            missing: vec![(
                RuleId::new("org.require-open-weights"),
                DataDimension::Openness
            )],
        }
    );
}

#[test]
fn deny_provider_bans_by_provider_id() {
    let model = fr_self_hosted_model("acme/model-x", "acme");
    let policy = Policy::new(vec![Rule::always(
        "org.deny-provider-acme",
        Constraint::DenyProvider(vec![ProviderId::new("acme")]),
    )]);

    assert_eq!(
        evaluate(&model, &policy, &any_need()),
        Verdict::Ineligible {
            violations: vec![RuleId::new("org.deny-provider-acme")],
        }
    );
}

#[test]
fn min_context_window_filters_short_context_models() {
    let policy = Policy::new(vec![Rule::always(
        "rulebook.long-context",
        Constraint::MinContextWindow(200_000),
    )]);

    let short = fr_self_hosted_model("acme/short-ctx", "acme").with_context_window(32_000);
    assert_eq!(
        evaluate(&short, &policy, &any_need()),
        Verdict::Ineligible {
            violations: vec![RuleId::new("rulebook.long-context")],
        }
    );

    let long = fr_self_hosted_model("acme/long-ctx", "acme").with_context_window(1_000_000);
    assert!(matches!(
        evaluate(&long, &policy, &any_need()),
        Verdict::Eligible { .. }
    ));
}

#[test]
fn min_bench_threshold_bans_below_and_unknown_is_indeterminate() {
    let policy = Policy::new(vec![Rule::always(
        "rulebook.coding-floor",
        Constraint::MinBench {
            dimension: BenchDimension::Coding,
            threshold: 40.0,
        },
    )]);

    let weak =
        fr_self_hosted_model("acme/weak-coder", "acme").with_bench(BenchDimension::Coding, 25.0);
    assert_eq!(
        evaluate(&weak, &policy, &any_need()),
        Verdict::Ineligible {
            violations: vec![RuleId::new("rulebook.coding-floor")],
        }
    );

    let unbenchmarked = fr_self_hosted_model("acme/unbenchmarked", "acme");
    assert_eq!(
        evaluate(&unbenchmarked, &policy, &any_need()),
        Verdict::Indeterminate {
            missing: vec![(
                RuleId::new("rulebook.coding-floor"),
                DataDimension::Bench(BenchDimension::Coding)
            )],
        }
    );

    let strong =
        fr_self_hosted_model("acme/strong-coder", "acme").with_bench(BenchDimension::Coding, 62.5);
    assert!(matches!(
        evaluate(&strong, &policy, &any_need()),
        Verdict::Eligible { .. }
    ));
}

#[test]
fn purpose_scoped_rule_applies_to_its_purpose_only() {
    let model = fr_self_hosted_model("acme/closed-model", "acme").with_openness(Openness::Closed);
    let policy = Policy::new(vec![Rule::when_purpose(
        Purpose::HealthData,
        "rulebook.health-requires-open-weights",
        Constraint::RequireOpenness(vec![Openness::OpenWeight, Openness::OpenSource]),
    )]);

    let health_need = NeedProfile::new(
        Task::SummaryExtraction,
        Purpose::HealthData,
        Sensitivity::C2,
    );
    assert_eq!(
        evaluate(&model, &policy, &health_need),
        Verdict::Ineligible {
            violations: vec![RuleId::new("rulebook.health-requires-open-weights")],
        }
    );

    let public_need = NeedProfile::new(
        Task::SummaryExtraction,
        Purpose::PublicContent,
        Sensitivity::C0,
    );
    assert!(matches!(
        evaluate(&model, &policy, &public_need),
        Verdict::Eligible { .. }
    ));
}

#[test]
fn task_scoped_rule_applies_to_its_task_only() {
    let model =
        fr_self_hosted_model("acme/small-coder", "acme").with_bench(BenchDimension::Coding, 30.0);
    let policy = Policy::new(vec![Rule::when_task(
        Task::CodeGeneration,
        "rulebook.code-floor",
        Constraint::MinBench {
            dimension: BenchDimension::Coding,
            threshold: 40.0,
        },
    )]);

    let code_need = NeedProfile::new(
        Task::CodeGeneration,
        Purpose::PublicContent,
        Sensitivity::C0,
    );
    assert_eq!(
        evaluate(&model, &policy, &code_need),
        Verdict::Ineligible {
            violations: vec![RuleId::new("rulebook.code-floor")],
        }
    );

    let chat_need = NeedProfile::new(Task::GeneralChat, Purpose::PublicContent, Sensitivity::C0);
    assert!(matches!(
        evaluate(&model, &policy, &chat_need),
        Verdict::Eligible { .. }
    ));
}

#[test]
fn ranking_sorts_by_task_dimension_then_price_and_unbenchmarked_last() {
    let strong_pricey = fr_self_hosted_model("acme/strong-pricey", "acme")
        .with_bench(BenchDimension::Coding, 70.0)
        .with_price(Price::per_mtok(5.0, 25.0));
    let mid_cheap = fr_self_hosted_model("acme/mid-cheap", "acme")
        .with_bench(BenchDimension::Coding, 50.0)
        .with_price(Price::per_mtok(0.5, 2.0));
    let mid_pricey = fr_self_hosted_model("acme/mid-pricey", "acme")
        .with_bench(BenchDimension::Coding, 50.0)
        .with_price(Price::per_mtok(3.0, 12.0));
    let unbenchmarked =
        fr_self_hosted_model("acme/unbenchmarked", "acme").with_price(Price::per_mtok(0.1, 0.4));

    let models = [&strong_pricey, &mid_cheap, &mid_pricey, &unbenchmarked];
    let spec = RankingSpec::new(vec![BenchDimension::Coding]);

    let ranked: Vec<&str> = rank(&models, &spec).iter().map(|m| m.id()).collect();

    assert_eq!(
        ranked,
        vec![
            "acme/strong-pricey",
            "acme/mid-cheap",
            "acme/mid-pricey",
            "acme/unbenchmarked",
        ]
    );
}

#[test]
fn unknown_model_is_ineligible_by_default() {
    let catalog = vec![fr_self_hosted_model("acme/known-model", "acme")];

    let verdict = verdict_for(
        &catalog,
        "acme/never-heard-of",
        &Policy::new(vec![]),
        &any_need(),
    );

    assert_eq!(
        verdict,
        Verdict::Ineligible {
            violations: vec![RuleId::builtin_unknown_model()],
        }
    );
}

#[test]
fn missing_origin_under_origin_rule_is_indeterminate() {
    let model = Model::new("unknown/mystery-model", "unknown").with_hosting(Hosting::SelfHosted);
    let policy = Policy::new(vec![Rule::always(
        "org.deny-origin-us-cn",
        Constraint::DenyOrigin(vec![CountryCode::new("US"), CountryCode::new("CN")]),
    )]);

    let verdict = evaluate(&model, &policy, &any_need());

    assert_eq!(
        verdict,
        Verdict::Indeterminate {
            missing: vec![(RuleId::new("org.deny-origin-us-cn"), DataDimension::Origin)],
        }
    );
}

#[test]
fn deny_origin_bans_matching_model() {
    let model = Model::new("deepseek/deepseek-v4", "deepseek")
        .with_origin(CountryCode::new("CN"))
        .with_hosting(Hosting::SelfHosted);
    let policy = Policy::new(vec![Rule::always(
        "org.deny-origin-us-cn",
        Constraint::DenyOrigin(vec![CountryCode::new("US"), CountryCode::new("CN")]),
    )]);

    let verdict = evaluate(&model, &policy, &any_need());

    assert_eq!(
        verdict,
        Verdict::Ineligible {
            violations: vec![RuleId::new("org.deny-origin-us-cn")],
        }
    );
}
