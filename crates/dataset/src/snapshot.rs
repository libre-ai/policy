//! Snapshot: manifest with dated sources, entries with per-field provenance.

use std::path::Path;

use serde::{Deserialize, Serialize};

use rumble_ai_clearance_domain::Model;

use crate::error::DatasetError;

/// Where a piece of snapshot data came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    ArtificialAnalysis,
    HuggingFace,
    Curated,
}

/// One dated source referenced by snapshot entries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceInfo {
    id: String,
    kind: SourceKind,
    fetched_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

impl SourceInfo {
    pub fn new(id: impl Into<String>, kind: SourceKind, fetched_at: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind,
            fetched_at: fetched_at.into(),
            note: None,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

/// Snapshot-level metadata: every eligibility decision cites this.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    schema_version: u32,
    generated_at: String,
    sources: Vec<SourceInfo>,
}

impl Manifest {
    pub fn new(generated_at: impl Into<String>, sources: Vec<SourceInfo>) -> Self {
        Self {
            schema_version: 1,
            generated_at: generated_at.into(),
            sources,
        }
    }

    pub fn generated_at(&self) -> &str {
        &self.generated_at
    }

    pub fn sources(&self) -> &[SourceInfo] {
        &self.sources
    }
}

/// Which manifest source backs each field group of a model entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    /// Identity and catalogue facts (id, licence, context, modalities).
    identity: String,
    /// Origin country, openness, self-hostability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    governance: Option<String>,
    /// Benchmark indices, price, speed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bench: Option<String>,
    /// Hosting paths and jurisdictions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hosting: Option<String>,
}

impl Provenance {
    pub fn new(identity: impl Into<String>) -> Self {
        Self {
            identity: identity.into(),
            governance: None,
            bench: None,
            hosting: None,
        }
    }

    pub fn with_governance(mut self, source: impl Into<String>) -> Self {
        self.governance = Some(source.into());
        self
    }

    pub fn with_bench(mut self, source: impl Into<String>) -> Self {
        self.bench = Some(source.into());
        self
    }

    pub fn with_hosting(mut self, source: impl Into<String>) -> Self {
        self.hosting = Some(source.into());
        self
    }

    fn referenced_sources(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.identity.as_str())
            .chain(self.governance.as_deref())
            .chain(self.bench.as_deref())
            .chain(self.hosting.as_deref())
    }
}

/// One model with its provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotEntry {
    model: Model,
    provenance: Provenance,
}

impl SnapshotEntry {
    pub fn new(model: Model, provenance: Provenance) -> Self {
        Self { model, provenance }
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    pub fn provenance(&self) -> &Provenance {
        &self.provenance
    }
}

/// A validated snapshot: the only thing the engine ever evaluates against.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    manifest: Manifest,
    entries: Vec<SnapshotEntry>,
}

impl Snapshot {
    /// Build and validate: duplicate model ids and dangling provenance
    /// references are errors — a snapshot is either fully sound or refused.
    pub fn new(manifest: Manifest, entries: Vec<SnapshotEntry>) -> Result<Self, DatasetError> {
        let snapshot = Self { manifest, entries };
        snapshot.validate()?;
        Ok(snapshot)
    }

    fn validate(&self) -> Result<(), DatasetError> {
        if self.manifest.schema_version != 1 {
            return Err(DatasetError::UnsupportedSchemaVersion(
                self.manifest.schema_version,
            ));
        }

        let mut seen: Vec<&str> = Vec::new();
        let source_ids: Vec<&str> = self
            .manifest
            .sources
            .iter()
            .map(|source| source.id.as_str())
            .collect();

        for entry in &self.entries {
            let id = entry.model.id();
            if seen.contains(&id) {
                return Err(DatasetError::DuplicateModelId(id.to_string()));
            }
            seen.push(id);

            for source in entry.provenance.referenced_sources() {
                if !source_ids.contains(&source) {
                    return Err(DatasetError::UnknownProvenanceSource {
                        model: id.to_string(),
                        source_id: source.to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn entries(&self) -> &[SnapshotEntry] {
        &self.entries
    }

    /// The catalogue the engine evaluates.
    pub fn models(&self) -> Vec<Model> {
        self.entries
            .iter()
            .map(|entry| entry.model.clone())
            .collect()
    }
}

/// Parse and validate a snapshot from JSON.
pub fn parse_snapshot(json: &str) -> Result<Snapshot, DatasetError> {
    let snapshot: Snapshot = serde_json::from_str(json)?;
    snapshot.validate()?;
    Ok(snapshot)
}

/// Load and validate a snapshot from disk.
pub fn load_snapshot(path: &Path) -> Result<Snapshot, DatasetError> {
    let raw = std::fs::read_to_string(path).map_err(|source| DatasetError::Io {
        path: path.display().to_string(),
        source,
    })?;
    parse_snapshot(&raw)
}

/// Write a snapshot atomically: temp file in the same directory, then rename.
/// A failed sync never replaces the previous snapshot.
pub fn write_snapshot_atomic(path: &Path, snapshot: &Snapshot) -> Result<(), DatasetError> {
    let json = serde_json::to_string_pretty(snapshot)?;
    let tmp = path.with_extension("json.tmp");
    let io_err = |source| DatasetError::Io {
        path: path.display().to_string(),
        source,
    };
    std::fs::write(&tmp, json).map_err(io_err)?;
    std::fs::rename(&tmp, path).map_err(io_err)?;
    Ok(())
}
