//! The eligibility evaluation itself: pure, deterministic, fail-closed.

use crate::model::{Hosting, Model};
use crate::need::NeedProfile;
use crate::policy::{Constraint, Policy, Rule, RuleId};
use crate::verdict::{DataDimension, Verdict};

/// Outcome of checking one rule against one model or hosting path.
enum RuleCheck {
    Satisfied,
    Violated,
    MissingData(DataDimension),
}

/// Fail-closed check of an optional model field: missing data is reported,
/// never assumed compliant.
fn check_known<T>(
    value: &Option<T>,
    dimension: DataDimension,
    satisfied: impl Fn(&T) -> bool,
) -> RuleCheck {
    match value {
        None => RuleCheck::MissingData(dimension),
        Some(value) if satisfied(value) => RuleCheck::Satisfied,
        Some(_) => RuleCheck::Violated,
    }
}

/// Check a model-scoped constraint. `None` when the constraint is
/// hosting-scoped and handled per deployment path instead.
fn check_model(constraint: &Constraint, model: &Model) -> Option<RuleCheck> {
    match constraint {
        Constraint::DenyOrigin(banned) => Some(check_known(
            &model.origin,
            DataDimension::Origin,
            |origin| !banned.contains(origin),
        )),
        Constraint::DenyProvider(banned) => Some(if banned.contains(&model.provider) {
            RuleCheck::Violated
        } else {
            RuleCheck::Satisfied
        }),
        Constraint::RequireOpenness(allowed) => Some(check_known(
            &model.openness,
            DataDimension::Openness,
            |openness| allowed.contains(openness),
        )),
        Constraint::RequireSelfHostable => Some(check_known(
            &model.self_hostable,
            DataDimension::SelfHostable,
            |hostable| *hostable,
        )),
        Constraint::MinContextWindow(minimum) => Some(check_known(
            &model.context_window,
            DataDimension::ContextWindow,
            |tokens| tokens >= minimum,
        )),
        Constraint::MinBench {
            dimension,
            threshold,
        } => Some(check_known(
            &model.bench_score(*dimension),
            DataDimension::Bench(*dimension),
            |score| score >= threshold,
        )),
        Constraint::DenyHostingJurisdiction(_) => None,
    }
}

/// Check a hosting-scoped constraint against one deployment path. `None`
/// when the constraint is model-scoped.
fn check_hosting(constraint: &Constraint, hosting: &Hosting) -> Option<RuleCheck> {
    match constraint {
        Constraint::DenyOrigin(_)
        | Constraint::DenyProvider(_)
        | Constraint::RequireOpenness(_)
        | Constraint::RequireSelfHostable
        | Constraint::MinContextWindow(_)
        | Constraint::MinBench { .. } => None,
        Constraint::DenyHostingJurisdiction(banned) => Some(match hosting {
            // Nothing leaves the organisation: vacuously satisfied.
            Hosting::SelfHosted => RuleCheck::Satisfied,
            Hosting::Api {
                jurisdiction: None, ..
            } => RuleCheck::MissingData(DataDimension::HostingJurisdiction),
            Hosting::Api {
                jurisdiction: Some(jurisdiction),
                ..
            } => {
                if banned.contains(jurisdiction) {
                    RuleCheck::Violated
                } else {
                    RuleCheck::Satisfied
                }
            }
        }),
    }
}

/// Evaluate one model against the effective policy for a given need.
///
/// Violations dominate missing data: a model already disqualified by one rule
/// is `Ineligible` even if other rules could not be checked. Hosting-scoped
/// rules filter deployment paths instead of disqualifying the model; the
/// model stays eligible while at least one compliant path remains.
pub fn evaluate(model: &Model, policy: &Policy, need: &NeedProfile) -> Verdict {
    let active_rules: Vec<&Rule> = policy
        .rules
        .iter()
        .filter(|rule| rule.is_active(need))
        .collect();

    let mut violations = Vec::new();
    let mut missing = Vec::new();

    for rule in &active_rules {
        match check_model(&rule.constraint, model) {
            None | Some(RuleCheck::Satisfied) => {}
            Some(RuleCheck::Violated) => violations.push(rule.id.clone()),
            Some(RuleCheck::MissingData(dimension)) => {
                missing.push((rule.id.clone(), dimension));
            }
        }
    }

    let mut viable_hostings = Vec::new();
    let mut hosting_blockers: Vec<RuleId> = Vec::new();
    let mut hosting_missing: Vec<(RuleId, DataDimension)> = Vec::new();

    for hosting in &model.hostings {
        let mut blocked = false;
        let mut unknown = false;
        for rule in &active_rules {
            match check_hosting(&rule.constraint, hosting) {
                None | Some(RuleCheck::Satisfied) => {}
                Some(RuleCheck::Violated) => {
                    blocked = true;
                    if !hosting_blockers.contains(&rule.id) {
                        hosting_blockers.push(rule.id.clone());
                    }
                }
                Some(RuleCheck::MissingData(dimension)) => {
                    unknown = true;
                    let entry = (rule.id.clone(), dimension);
                    if !hosting_missing.contains(&entry) {
                        hosting_missing.push(entry);
                    }
                }
            }
        }
        if !blocked && !unknown {
            viable_hostings.push(hosting.clone());
        }
    }

    if viable_hostings.is_empty() {
        // Fail-closed: no compliant deployment path. Definitive blocks yield
        // Ineligible; unknowns yield Indeterminate; a model with no
        // documented path at all is missing data, never eligible.
        if model.hostings.is_empty() {
            missing.push((RuleId::builtin_hosting_paths(), DataDimension::HostingPaths));
        } else if hosting_missing.is_empty() {
            violations.extend(hosting_blockers);
        } else {
            missing.extend(hosting_missing);
        }
    }

    if !violations.is_empty() {
        Verdict::Ineligible { violations }
    } else if !missing.is_empty() {
        Verdict::Indeterminate { missing }
    } else {
        Verdict::Eligible { viable_hostings }
    }
}

/// Verdict for a model looked up by id: a model absent from the snapshot is
/// ineligible, never silently accepted.
pub fn verdict_for(
    catalog: &[Model],
    model_id: &str,
    policy: &Policy,
    need: &NeedProfile,
) -> Verdict {
    match catalog.iter().find(|model| model.id == model_id) {
        Some(model) => evaluate(model, policy, need),
        None => Verdict::Ineligible {
            violations: vec![RuleId::builtin_unknown_model()],
        },
    }
}
