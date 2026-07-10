//! Snapshot builders: Artificial Analysis (org's own key), Hugging Face and
//! curated governance connectors, merged with atomic writes.
//!
//! AA data is internal-use-only (free tier terms): it is written to the
//! gitignored `data/` directory and must never be committed or served
//! publicly. Connectors parse recorded responses; live fetching happens at
//! the CLI edge so tests never touch the network.
#![forbid(unsafe_code)]

mod aa;
mod error;
mod hf;
mod merge;

pub use aa::{AaModel, parse_aa_response};
pub use error::SyncError;
pub use hf::{HfModel, parse_hf_response};
pub use merge::{SyncTimestamps, build_snapshot};

/// Free-tier Artificial Analysis models endpoint (auth: `x-api-key` header).
pub const AA_FREE_MODELS_URL: &str = "https://artificialanalysis.ai/api/v2/language/models/free";

/// Hugging Face models listing for one organisation (public, no auth).
pub fn hf_models_url(org: &str) -> String {
    format!("https://huggingface.co/api/models?author={org}&pipeline_tag=text-generation&limit=100")
}
