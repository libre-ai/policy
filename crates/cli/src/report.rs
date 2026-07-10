//! Report shapes: JSON envelope `{ data, meta }` and text rendering.

use serde::Serialize;

use rumble_ai_clearance_domain::{ApiKind, Hosting, Verdict};

#[derive(Debug, Serialize)]
pub struct EvaluateReport {
    pub data: EvaluateData,
    pub meta: Meta,
}

#[derive(Debug, Serialize)]
pub struct EvaluateData {
    /// Ranked: task bench dimensions, then blended price, then id.
    pub eligible: Vec<EligibleEntry>,
    pub ineligible_count: usize,
    pub indeterminate_count: usize,
}

#[derive(Debug, Serialize)]
pub struct EligibleEntry {
    pub model: String,
    pub viable_hostings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Meta {
    pub snapshot_generated_at: String,
    pub source_count: usize,
}

/// Compact human-readable label for a deployment path.
pub fn hosting_labels(hostings: &[Hosting]) -> Vec<String> {
    hostings
        .iter()
        .map(|hosting| match hosting {
            Hosting::SelfHosted => "self-hosted".to_string(),
            Hosting::Api {
                kind, jurisdiction, ..
            } => {
                let kind = match kind {
                    ApiKind::EuSovereign => "eu-sovereign-api",
                    ApiKind::Provider => "provider-api",
                    ApiKind::Hyperscaler => "hyperscaler-api",
                };
                match jurisdiction {
                    Some(country) => format!("{kind}({country:?})"),
                    None => format!("{kind}(jurisdiction unknown)"),
                }
            }
        })
        .collect()
}

/// Rule-by-rule text rendering of a verdict.
pub fn verdict_lines(verdict: &Verdict) -> Vec<String> {
    match verdict {
        Verdict::Eligible { viable_hostings } => {
            let mut lines = vec!["verdict: ELIGIBLE".to_string()];
            for label in hosting_labels(viable_hostings) {
                lines.push(format!("  viable hosting: {label}"));
            }
            lines
        }
        Verdict::Ineligible { violations } => {
            let mut lines = vec!["verdict: INELIGIBLE".to_string()];
            for rule in violations {
                lines.push(format!("  violates: {rule:?}"));
            }
            lines
        }
        Verdict::Indeterminate { missing } => {
            let mut lines = vec!["verdict: INDETERMINATE (missing data, fail-closed)".to_string()];
            for (rule, dimension) in missing {
                lines.push(format!("  missing: {dimension:?} (required by {rule:?})"));
            }
            lines
        }
    }
}
