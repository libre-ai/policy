//! Sync errors. Fail-closed: a failed sync never yields a snapshot.

use rumble_ai_clearance_dataset::DatasetError;

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("invalid source JSON: {source}")]
    Json {
        #[from]
        source: serde_json::Error,
    },
    #[error(transparent)]
    Dataset(#[from] DatasetError),
}
