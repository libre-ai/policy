//! Hugging Face connector: response parsing only — fetching happens at the
//! CLI edge, tests run on recorded fixtures.
//!
//! Public endpoint, as called by `hf_models_url`:
//! `GET https://huggingface.co/api/models?author=<org>&pipeline_tag=text-generation&limit=100`.
//! The cap is ours: an organisation with more than 100 listings is truncated,
//! so a model can be absent from the snapshot — and an absent model is
//! ineligible, never silently accepted.

use serde::Deserialize;

use crate::error::SyncError;

#[derive(Debug, Deserialize)]
struct HfModelDoc {
    id: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default, deserialize_with = "gated_flag")]
    gated: bool,
    #[serde(default)]
    downloads: u64,
}

/// HF reports gating as `false` or a mode string ("auto", "manual").
/// Fail-closed: any unrecognised shape counts as gated.
fn gated_flag<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(match value {
        serde_json::Value::Bool(flag) => flag,
        serde_json::Value::Null => false,
        _ => true,
    })
}

/// One open-weights model as listed by Hugging Face.
#[derive(Debug, Clone, PartialEq)]
pub struct HfModel {
    repo_id: String,
    license: Option<String>,
    gated: bool,
    downloads: u64,
}

impl HfModel {
    /// Repository id, e.g. `mistralai/Mistral-Large-3`.
    pub fn repo_id(&self) -> &str {
        &self.repo_id
    }

    /// Organisation handle, e.g. `mistralai`.
    pub fn org(&self) -> &str {
        self.repo_id.split('/').next().unwrap_or(&self.repo_id)
    }

    /// Repository name without the organisation.
    pub fn repo_name(&self) -> &str {
        self.repo_id
            .split_once('/')
            .map_or(self.repo_id.as_str(), |(_, name)| name)
    }

    /// SPDX-ish licence id from the `license:` tag, if any.
    pub fn license(&self) -> Option<&str> {
        self.license.as_deref()
    }

    pub fn gated(&self) -> bool {
        self.gated
    }

    pub fn downloads(&self) -> u64 {
        self.downloads
    }
}

/// Parse a Hugging Face models listing.
pub fn parse_hf_response(json: &str) -> Result<Vec<HfModel>, SyncError> {
    let docs: Vec<HfModelDoc> = serde_json::from_str(json)?;
    Ok(docs
        .into_iter()
        .map(|doc| {
            let license = doc
                .tags
                .iter()
                .find_map(|tag| tag.strip_prefix("license:"))
                .map(str::to_string);
            HfModel {
                repo_id: doc.id,
                license,
                gated: doc.gated,
                downloads: doc.downloads,
            }
        })
        .collect())
}
