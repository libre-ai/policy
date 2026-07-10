//! Artificial Analysis connector: response parsing only — fetching happens
//! at the CLI edge, tests run on recorded fixtures.
//!
//! Free-tier endpoint: `GET /api/v2/language/models/free` with an
//! `x-api-key` header. AA data is internal-use-only: parsed models feed the
//! org-local snapshot and are never redistributed.

use serde::Deserialize;

use rumble_ai_clearance_domain::BenchDimension;

use crate::error::SyncError;

#[derive(Debug, Deserialize)]
struct AaResponse {
    data: Vec<AaModelDoc>,
}

#[derive(Debug, Deserialize)]
struct AaModelDoc {
    slug: String,
    #[allow(dead_code)]
    name: String,
    model_creator: AaCreatorDoc,
    #[serde(default)]
    evaluations: AaEvaluationsDoc,
    #[serde(default)]
    pricing: AaPricingDoc,
}

#[derive(Debug, Deserialize)]
struct AaCreatorDoc {
    name: String,
    #[serde(default)]
    country: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct AaEvaluationsDoc {
    #[serde(default)]
    artificial_analysis_intelligence_index: Option<f64>,
    #[serde(default)]
    artificial_analysis_coding_index: Option<f64>,
    #[serde(default)]
    artificial_analysis_agentic_index: Option<f64>,
    #[serde(default)]
    artificial_analysis_math_index: Option<f64>,
    #[serde(default)]
    artificial_analysis_multilingual_index: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
struct AaPricingDoc {
    #[serde(default)]
    price_1m_input_tokens: Option<f64>,
    #[serde(default)]
    price_1m_output_tokens: Option<f64>,
}

/// One model as reported by Artificial Analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct AaModel {
    slug: String,
    creator_name: String,
    creator_country: Option<String>,
    intelligence: Option<f64>,
    coding: Option<f64>,
    agentic: Option<f64>,
    math: Option<f64>,
    multilingual: Option<f64>,
    price_input: Option<f64>,
    price_output: Option<f64>,
}

impl AaModel {
    pub fn slug(&self) -> &str {
        &self.slug
    }

    pub fn creator_name(&self) -> &str {
        &self.creator_name
    }

    /// AA's own creator country — corroboration only; the curated governance
    /// dataset stays the authority for origin.
    pub fn creator_country(&self) -> Option<&str> {
        self.creator_country.as_deref()
    }

    /// Missing indices stay unknown, never zero (fail-closed downstream).
    pub fn index(&self, dimension: BenchDimension) -> Option<f64> {
        match dimension {
            BenchDimension::Intelligence => self.intelligence,
            BenchDimension::Coding => self.coding,
            BenchDimension::Agentic => self.agentic,
            BenchDimension::Math => self.math,
            BenchDimension::Multilingual => self.multilingual,
        }
    }

    pub fn price_input(&self) -> Option<f64> {
        self.price_input
    }

    pub fn price_output(&self) -> Option<f64> {
        self.price_output
    }
}

/// Parse an AA models response (free or pro tier shape).
pub fn parse_aa_response(json: &str) -> Result<Vec<AaModel>, SyncError> {
    let response: AaResponse = serde_json::from_str(json)?;
    Ok(response
        .data
        .into_iter()
        .map(|doc| AaModel {
            slug: doc.slug,
            creator_name: doc.model_creator.name,
            creator_country: doc.model_creator.country,
            intelligence: doc.evaluations.artificial_analysis_intelligence_index,
            coding: doc.evaluations.artificial_analysis_coding_index,
            agentic: doc.evaluations.artificial_analysis_agentic_index,
            math: doc.evaluations.artificial_analysis_math_index,
            multilingual: doc.evaluations.artificial_analysis_multilingual_index,
            price_input: doc.pricing.price_1m_input_tokens,
            price_output: doc.pricing.price_1m_output_tokens,
        })
        .collect())
}
