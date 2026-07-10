//! Explainable per-model verdicts.

use serde::{Deserialize, Serialize};

use crate::model::{BenchDimension, Hosting};
use crate::policy::RuleId;

/// A model data dimension a rule needed but the snapshot does not provide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataDimension {
    Origin,
    HostingJurisdiction,
    HostingPaths,
    Openness,
    SelfHostable,
    ContextWindow,
    Bench(BenchDimension),
}

/// Rule-by-rule explainable verdict for one model.
///
/// `Indeterminate` is fail-closed: it is never treated as eligible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Eligible {
        /// Deployment paths that satisfy every hosting-scoped rule.
        viable_hostings: Vec<Hosting>,
    },
    Ineligible {
        violations: Vec<RuleId>,
    },
    Indeterminate {
        missing: Vec<(RuleId, DataDimension)>,
    },
}
