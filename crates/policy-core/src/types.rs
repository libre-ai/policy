// RFC 8785 JSON Canonicalization, RFC 2119 keywords, SEMANTICS.md §3-7

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    InputInvalid,
    EvaluatedAtInvalid,
    RuleIdDuplicate,
    ApprovalInvalid,
    DigestMismatch,
    TenantMismatch,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InputInvalid => "input-invalid",
            Self::EvaluatedAtInvalid => "evaluated-at-invalid",
            Self::RuleIdDuplicate => "rule-id-duplicate",
            Self::ApprovalInvalid => "approval-invalid",
            Self::DigestMismatch => "digest-mismatch",
            Self::TenantMismatch => "tenant-mismatch",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleStatus {
    Satisfied,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ReasonCode {
    RuleSatisfied,
    RuleFailed,
    SourceFromFuture,
    SnapshotStale,
    FactTypeMismatch,
    FactAbsent,
}

impl ReasonCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RuleSatisfied => "policy.rule_satisfied",
            Self::RuleFailed => "policy.rule_failed",
            Self::SourceFromFuture => "policy.source_from_future",
            Self::SnapshotStale => "policy.snapshot_stale",
            Self::FactTypeMismatch => "policy.fact_type_mismatch",
            Self::FactAbsent => "policy.fact_absent",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Eligible,
    Ineligible,
    Indeterminate,
}

impl Verdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Eligible => "eligible",
            Self::Ineligible => "ineligible",
            Self::Indeterminate => "indeterminate",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Equals,
    NotEquals,
    In,
    NotIn,
    AtLeast,
    AtMost,
}

impl Operator {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "equals" => Some(Self::Equals),
            "not-equals" => Some(Self::NotEquals),
            "in" => Some(Self::In),
            "not-in" => Some(Self::NotIn),
            "at-least" => Some(Self::AtLeast),
            "at-most" => Some(Self::AtMost),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Equals => "equals",
            Self::NotEquals => "not-equals",
            Self::In => "in",
            Self::NotIn => "not-in",
            Self::AtLeast => "at-least",
            Self::AtMost => "at-most",
        }
    }
}

// Rule value can be scalar or array of scalars
#[derive(Debug, Clone, PartialEq)]
pub enum RuleValue {
    String(String),
    Number(f64),
    Boolean(bool),
    StringArray(Vec<String>),
    NumberArray(Vec<f64>),
    BooleanArray(Vec<bool>),
}

// Fact value is always scalar
#[derive(Debug, Clone, PartialEq)]
pub enum FactValue {
    String(String),
    Number(f64),
    Boolean(bool),
}

impl FactValue {
    pub fn type_rank(&self) -> u8 {
        match self {
            Self::Boolean(_) => 0,
            Self::Number(_) => 1,
            Self::String(_) => 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleResult {
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    pub status: String,
    #[serde(rename = "reasonCode")]
    pub reason_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvaluation {
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    pub id: String,
    #[serde(rename = "tenantId")]
    pub tenant_id: String,
    #[serde(rename = "policyId")]
    pub policy_id: String,
    #[serde(rename = "policyDigest")]
    pub policy_digest: String,
    #[serde(rename = "snapshotId")]
    pub snapshot_id: String,
    #[serde(rename = "snapshotDigest")]
    pub snapshot_digest: String,
    #[serde(rename = "needDigest")]
    pub need_digest: String,
    #[serde(rename = "engineVersion")]
    pub engine_version: String,
    pub verdict: String,
    #[serde(rename = "ruleResults")]
    pub rule_results: Vec<RuleResult>,
    #[serde(rename = "evaluatedAt")]
    pub evaluated_at: String,
    pub digest: String,
}

pub const ENGINE_VERSION: &str = "2.0.0";

pub const INPUT_LIMIT_POLICY: usize = 8 * 1024 * 1024;
pub const INPUT_LIMIT_SNAPSHOT: usize = 8 * 1024 * 1024;
pub const INPUT_LIMIT_NEED: usize = 8 * 1024 * 1024;
pub const INPUT_LIMIT_EVALUATED_AT: usize = 20;
pub const OUTPUT_LIMIT_SUCCESS: usize = 2 * 1024 * 1024;
