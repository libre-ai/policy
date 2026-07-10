//! Pure eligibility engine: types and verdict logic, zero I/O.
//!
//! Compiled both natively (CLI, API) and to WASM (web UI local mode), so this
//! crate must stay free of platform-specific dependencies.
//!
//! Core principles, enforced by tests and property-based invariants:
//! - **Deny-by-default**: unknown models and missing data are never eligible.
//! - **Explainability**: every verdict is attributable rule by rule.
//! - **Filter then rank**: eligibility (security's domain) is evaluated
//!   strictly apart from preference ordering (business's domain).
#![forbid(unsafe_code)]

mod engine;
mod model;
mod need;
mod policy;
mod ranking;
mod verdict;

pub use engine::{evaluate, verdict_for};
pub use model::{
    ApiKind, BenchDimension, CountryCode, Hosting, Model, Openness, Price, ProviderId,
};
pub use need::{NeedProfile, Purpose, Sensitivity, Task};
pub use policy::{Applicability, Constraint, Policy, Rule, RuleId};
pub use ranking::{RankingSpec, rank};
pub use verdict::{DataDimension, Verdict};
