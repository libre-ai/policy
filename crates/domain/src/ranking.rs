//! Preference ordering of already-eligible models.

use serde::{Deserialize, Serialize};

use crate::model::{BenchDimension, Model};

/// How to order eligible models: benchmark dimensions in priority order.
///
/// Ranking is preference, never eligibility — it only ever reorders models
/// the policy already cleared.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RankingSpec {
    dimensions: Vec<BenchDimension>,
}

impl RankingSpec {
    pub fn new(dimensions: Vec<BenchDimension>) -> Self {
        Self { dimensions }
    }
}

/// Rank eligible models: spec dimensions (descending, unbenchmarked last),
/// then blended price (ascending, unpriced last), then id for determinism.
pub fn rank<'a>(models: &[&'a Model], spec: &RankingSpec) -> Vec<&'a Model> {
    let mut ranked = models.to_vec();
    ranked.sort_by(|a, b| {
        for dimension in &spec.dimensions {
            let ordering =
                compare_scores_desc(a.bench_score(*dimension), b.bench_score(*dimension));
            if ordering != std::cmp::Ordering::Equal {
                return ordering;
            }
        }
        let price_ordering =
            compare_prices_asc(a.price.map(|p| p.blended()), b.price.map(|p| p.blended()));
        if price_ordering != std::cmp::Ordering::Equal {
            return price_ordering;
        }
        a.id.cmp(&b.id)
    });
    ranked
}

/// Higher scores first; models without a score always sort last.
fn compare_scores_desc(a: Option<f64>, b: Option<f64>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(a), Some(b)) => b.total_cmp(&a),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

/// Cheaper first; models without a price always sort last.
fn compare_prices_asc(a: Option<f64>, b: Option<f64>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.total_cmp(&b),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}
