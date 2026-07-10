//! Dataset errors. Fail-closed: any error means no snapshot at all.

/// Why a snapshot or governance document was refused.
#[derive(Debug, thiserror::Error)]
pub enum DatasetError {
    #[error("invalid snapshot JSON: {source}")]
    Json {
        #[from]
        source: serde_json::Error,
    },
    #[error("invalid governance YAML: {source}")]
    Yaml {
        #[from]
        source: yaml_serde::Error,
    },
    #[error("snapshot I/O error at {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("unsupported snapshot schema version {0} (expected 1)")]
    UnsupportedSchemaVersion(u32),
    #[error("unsupported governance version {0} (expected 1)")]
    UnsupportedGovernanceVersion(u32),
    #[error("duplicate model id `{0}` in snapshot")]
    DuplicateModelId(String),
    #[error("model `{model}` references unknown provenance source `{source_id}`")]
    UnknownProvenanceSource { model: String, source_id: String },
    #[error("duplicate provider id `{0}` in governance data")]
    DuplicateProviderId(String),
    #[error(
        "provider `{provider}` has invalid country code `{code}` (expected ISO 3166-1 alpha-2)"
    )]
    InvalidCountryCode { provider: String, code: String },
}
