//! Rules, constraints and the compiled effective policy.

use serde::{Deserialize, Serialize};

use crate::model::{BenchDimension, CountryCode, Openness, ProviderId};
use crate::need::{NeedProfile, Purpose, Sensitivity, Task};

/// Identifier of a policy rule, e.g. `org.deny-origin-us-cn`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuleId(String);

impl RuleId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Reserved id for the structural "no documented hosting path" gap —
    /// not a policy rule, but every verdict entry must stay attributable.
    pub fn builtin_hosting_paths() -> Self {
        Self::new("builtin.hosting-paths")
    }

    /// Reserved id for "model absent from the snapshot": deny-by-default.
    pub fn builtin_unknown_model() -> Self {
        Self::new("builtin.unknown-model")
    }
}

/// A single checkable constraint on a model.
///
/// Model-scoped constraints disqualify the model itself; hosting-scoped
/// constraints (`DenyHostingJurisdiction`) filter its deployment paths — the
/// model stays eligible if at least one compliant path remains.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Constraint {
    /// The model creator's country must not be one of these.
    DenyOrigin(Vec<CountryCode>),
    /// No inference data may flow under one of these jurisdictions.
    DenyHostingJurisdiction(Vec<CountryCode>),
    /// Inference data may only flow under one of these jurisdictions.
    /// Self-hosted paths satisfy this structurally; an empty list therefore
    /// means "self-host only".
    RequireHostingJurisdictionIn(Vec<CountryCode>),
    /// These providers are banned outright.
    DenyProvider(Vec<ProviderId>),
    /// The model's openness must be one of these.
    RequireOpenness(Vec<Openness>),
    /// The model must be deployable on the org's own infrastructure.
    RequireSelfHostable,
    /// Context window must be at least this many tokens.
    MinContextWindow(u64),
    /// Benchmark score on `dimension` must be at least `threshold`.
    MinBench {
        dimension: BenchDimension,
        threshold: f64,
    },
}

/// When a rule is active, relative to the need profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Applicability {
    Always,
    SensitivityAtLeast(Sensitivity),
    Purpose(Purpose),
    Task(Task),
}

/// A policy rule: an identified constraint, active for some need profiles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub(crate) id: RuleId,
    pub(crate) constraint: Constraint,
    pub(crate) applicability: Applicability,
}

impl Rule {
    /// A rule that applies whatever the need profile.
    pub fn always(id: impl Into<String>, constraint: Constraint) -> Self {
        Self {
            id: RuleId::new(id),
            constraint,
            applicability: Applicability::Always,
        }
    }

    /// A rule active only when the need's sensitivity reaches `level`.
    pub fn when_sensitivity_at_least(
        level: Sensitivity,
        id: impl Into<String>,
        constraint: Constraint,
    ) -> Self {
        Self {
            id: RuleId::new(id),
            constraint,
            applicability: Applicability::SensitivityAtLeast(level),
        }
    }

    /// A rule active only for this processing purpose.
    pub fn when_purpose(purpose: Purpose, id: impl Into<String>, constraint: Constraint) -> Self {
        Self {
            id: RuleId::new(id),
            constraint,
            applicability: Applicability::Purpose(purpose),
        }
    }

    /// A rule active only for this task.
    pub fn when_task(task: Task, id: impl Into<String>, constraint: Constraint) -> Self {
        Self {
            id: RuleId::new(id),
            constraint,
            applicability: Applicability::Task(task),
        }
    }

    pub(crate) fn is_active(&self, need: &NeedProfile) -> bool {
        match self.applicability {
            Applicability::Always => true,
            Applicability::SensitivityAtLeast(level) => need.sensitivity >= level,
            Applicability::Purpose(purpose) => need.purpose == purpose,
            Applicability::Task(task) => need.task == task,
        }
    }
}

/// The compiled, effective policy the engine evaluates against.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Policy {
    pub(crate) rules: Vec<Rule>,
}

impl Policy {
    pub fn new(rules: Vec<Rule>) -> Self {
        Self { rules }
    }

    pub fn with_rule(mut self, rule: Rule) -> Self {
        self.rules.push(rule);
        self
    }
}
