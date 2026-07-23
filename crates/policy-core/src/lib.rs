// policy-core: Pure policy evaluation engine
// SEMANTICS.md: https://github.com/libre-ai/libre-ai/blob/main/contracts/wit/policy-core-v2/SEMANTICS.md

pub mod evaluator;
pub mod jcs;
pub mod normalize;
pub mod types;

pub use evaluator::{EvaluateResult, evaluate};
pub use types::{ENGINE_VERSION, ErrorCode, ReasonCode, RuleStatus, Verdict};
