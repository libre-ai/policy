//! Property-based invariants of the eligibility engine.
//!
//! 1. Monotonicity: adding a rule can only shrink the eligible set.
//! 2. Deny-by-default: missing data on a required dimension is never eligible.
//! 3. Determinism: same inputs, same verdict.

use proptest::prelude::*;
use rumble_ai_clearance_domain::{
    ApiKind, BenchDimension, Constraint, CountryCode, Hosting, Model, NeedProfile, Openness,
    Policy, Purpose, Rule, Sensitivity, Task, Verdict, evaluate,
};

fn country() -> impl Strategy<Value = CountryCode> {
    prop_oneof![
        Just(CountryCode::new("US")),
        Just(CountryCode::new("CN")),
        Just(CountryCode::new("FR")),
        Just(CountryCode::new("DE")),
        Just(CountryCode::new("GB")),
    ]
}

fn openness() -> impl Strategy<Value = Openness> {
    prop_oneof![
        Just(Openness::Closed),
        Just(Openness::OpenWeight),
        Just(Openness::OpenSource),
    ]
}

fn api_kind() -> impl Strategy<Value = ApiKind> {
    prop_oneof![
        Just(ApiKind::EuSovereign),
        Just(ApiKind::Provider),
        Just(ApiKind::Hyperscaler),
    ]
}

fn hosting() -> impl Strategy<Value = Hosting> {
    prop_oneof![
        Just(Hosting::SelfHosted),
        (
            api_kind(),
            proptest::option::of(country()),
            proptest::option::of(country())
        )
            .prop_map(|(kind, country, jurisdiction)| Hosting::Api {
                kind,
                country,
                jurisdiction,
            }),
    ]
}

fn model() -> impl Strategy<Value = Model> {
    (
        "[a-z]{3,8}/[a-z0-9-]{3,12}",
        "[a-z]{3,8}",
        proptest::option::of(country()),
        proptest::option::of(openness()),
        proptest::option::of(any::<bool>()),
        proptest::option::of(1_000u64..2_000_000),
        proptest::collection::vec(hosting(), 0..3),
        proptest::option::of(0.0f64..100.0),
    )
        .prop_map(
            |(id, provider, origin, openness, self_hostable, context, hostings, coding)| {
                let mut model = Model::new(id, provider);
                if let Some(origin) = origin {
                    model = model.with_origin(origin);
                }
                if let Some(openness) = openness {
                    model = model.with_openness(openness);
                }
                if let Some(self_hostable) = self_hostable {
                    model = model.with_self_hostable(self_hostable);
                }
                if let Some(context) = context {
                    model = model.with_context_window(context);
                }
                for hosting in hostings {
                    model = model.with_hosting(hosting);
                }
                if let Some(coding) = coding {
                    model = model.with_bench(BenchDimension::Coding, coding);
                }
                model
            },
        )
}

fn constraint() -> impl Strategy<Value = Constraint> {
    prop_oneof![
        proptest::collection::vec(country(), 1..3).prop_map(Constraint::DenyOrigin),
        proptest::collection::vec(country(), 1..3).prop_map(Constraint::DenyHostingJurisdiction),
        proptest::collection::vec(openness(), 1..3).prop_map(Constraint::RequireOpenness),
        Just(Constraint::RequireSelfHostable),
        (1_000u64..500_000).prop_map(Constraint::MinContextWindow),
        (0.0f64..100.0).prop_map(|threshold| Constraint::MinBench {
            dimension: BenchDimension::Coding,
            threshold,
        }),
    ]
}

fn sensitivity() -> impl Strategy<Value = Sensitivity> {
    prop_oneof![
        Just(Sensitivity::C0),
        Just(Sensitivity::C1),
        Just(Sensitivity::C2),
        Just(Sensitivity::C3),
    ]
}

fn rule(index: usize) -> impl Strategy<Value = Rule> {
    (constraint(), sensitivity()).prop_map(move |(constraint, level)| {
        // Mix always-on and sensitivity-scoped rules.
        if index.is_multiple_of(2) {
            Rule::always(format!("prop.rule-{index}"), constraint)
        } else {
            Rule::when_sensitivity_at_least(level, format!("prop.rule-{index}"), constraint)
        }
    })
}

fn policy(max_rules: usize) -> impl Strategy<Value = Policy> {
    proptest::collection::vec(constraint(), 0..max_rules).prop_map(|constraints| {
        Policy::new(
            constraints
                .into_iter()
                .enumerate()
                .map(|(index, constraint)| Rule::always(format!("prop.rule-{index}"), constraint))
                .collect(),
        )
    })
}

fn need() -> impl Strategy<Value = NeedProfile> {
    sensitivity().prop_map(|sensitivity| {
        NeedProfile::new(Task::GeneralChat, Purpose::PublicContent, sensitivity)
    })
}

fn is_eligible(verdict: &Verdict) -> bool {
    matches!(verdict, Verdict::Eligible { .. })
}

proptest! {
    /// Adding a rule can only shrink the eligible set.
    #[test]
    fn adding_a_rule_never_creates_eligibility(
        model in model(),
        base in policy(3),
        extra in rule(97),
        need in need(),
    ) {
        let extended = base.clone().with_rule(extra);
        let base_verdict = evaluate(&model, &base, &need);
        let extended_verdict = evaluate(&model, &extended, &need);

        if is_eligible(&extended_verdict) {
            prop_assert!(
                is_eligible(&base_verdict),
                "extended policy eligible but base policy was {base_verdict:?}"
            );
        }
    }

    /// A model stripped of every optional datum is never eligible under a
    /// policy with at least one active rule (fail-closed), nor under any
    /// policy at all (no documented hosting path).
    #[test]
    fn bare_model_is_never_eligible(
        base in policy(3),
        need in need(),
    ) {
        let bare = Model::new("prop/bare-model", "prop");
        let verdict = evaluate(&bare, &base, &need);
        prop_assert!(!is_eligible(&verdict), "bare model got {verdict:?}");
    }

    /// Same inputs, same verdict.
    #[test]
    fn evaluation_is_deterministic(
        model in model(),
        base in policy(4),
        need in need(),
    ) {
        let first = evaluate(&model, &base, &need);
        let second = evaluate(&model, &base, &need);
        prop_assert_eq!(first, second);
    }
}
