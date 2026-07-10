//! Snapshot schema, manifest, per-field provenance, validated loading.
//!
//! A snapshot is either fully sound or refused — the engine never sees a
//! partially valid dataset. Writes are atomic: a failed sync never replaces
//! the previous snapshot.
#![forbid(unsafe_code)]

mod error;
mod governance;
mod snapshot;

pub use error::DatasetError;
pub use governance::{Governance, ProviderInfo, parse_governance};
pub use snapshot::{
    Manifest, Provenance, Snapshot, SnapshotEntry, SourceInfo, SourceKind, load_snapshot,
    parse_snapshot, write_snapshot_atomic,
};
