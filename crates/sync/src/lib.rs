//! Snapshot builders: Artificial Analysis (org's own key), Hugging Face and
//! curated governance connectors, merged with atomic writes.
//!
//! AA data is internal-use-only (free tier terms): it is written to the
//! gitignored `data/` directory and must never be committed or served
//! publicly.
#![forbid(unsafe_code)]
