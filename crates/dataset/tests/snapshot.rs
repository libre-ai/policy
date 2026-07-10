//! Snapshot format: manifest, provenance, validated loading, atomic writes.

use rumble_ai_clearance_dataset::{
    DatasetError, Manifest, Provenance, Snapshot, SnapshotEntry, SourceInfo, SourceKind,
    load_snapshot, parse_snapshot, write_snapshot_atomic,
};
use rumble_ai_clearance_domain::{CountryCode, Hosting, Model};

fn sample_manifest() -> Manifest {
    Manifest::new(
        "2026-07-10T12:00:00Z",
        vec![
            SourceInfo::new(
                "aa-2026-07-10",
                SourceKind::ArtificialAnalysis,
                "2026-07-10T11:58:00Z",
            ),
            SourceInfo::new(
                "hf-2026-07-10",
                SourceKind::HuggingFace,
                "2026-07-10T11:59:00Z",
            ),
            SourceInfo::new("curated-v1", SourceKind::Curated, "2026-07-10T00:00:00Z"),
        ],
    )
}

fn sample_entry(id: &str) -> SnapshotEntry {
    SnapshotEntry::new(
        Model::new(id, "mistralai")
            .with_origin(CountryCode::new("FR"))
            .with_hosting(Hosting::SelfHosted),
        Provenance::new("hf-2026-07-10")
            .with_governance("curated-v1")
            .with_bench("aa-2026-07-10"),
    )
}

#[test]
fn snapshot_round_trips_through_disk() {
    let snapshot = Snapshot::new(
        sample_manifest(),
        vec![sample_entry("mistralai/mistral-large-3")],
    )
    .expect("valid snapshot");

    let dir = std::env::temp_dir().join("rumble-ai-clearance-test-roundtrip");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("snapshot.json");

    write_snapshot_atomic(&path, &snapshot).expect("writes");
    let loaded = load_snapshot(&path).expect("loads");

    assert_eq!(loaded, snapshot);
    assert_eq!(loaded.models().len(), 1);
    assert_eq!(loaded.manifest().generated_at(), "2026-07-10T12:00:00Z");

    // No temp residue next to the snapshot.
    let residue: Vec<_> = std::fs::read_dir(&dir)
        .expect("read dir")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name() != "snapshot.json")
        .collect();
    assert!(residue.is_empty(), "unexpected residue: {residue:?}");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn duplicate_model_ids_are_rejected() {
    let result = Snapshot::new(
        sample_manifest(),
        vec![sample_entry("acme/twin"), sample_entry("acme/twin")],
    );

    assert!(matches!(
        result,
        Err(DatasetError::DuplicateModelId(id)) if id == "acme/twin"
    ));
}

#[test]
fn provenance_must_reference_a_manifest_source() {
    let entry = SnapshotEntry::new(
        Model::new("acme/orphan", "acme").with_hosting(Hosting::SelfHosted),
        Provenance::new("source-that-does-not-exist"),
    );

    let result = Snapshot::new(sample_manifest(), vec![entry]);

    assert!(matches!(
        result,
        Err(DatasetError::UnknownProvenanceSource { model, source_id })
            if model == "acme/orphan" && source_id == "source-that-does-not-exist"
    ));
}

#[test]
fn unsupported_schema_version_is_rejected() {
    let json = r#"{"manifest":{"schema_version":99,"generated_at":"2026-07-10T12:00:00Z","sources":[]},"entries":[]}"#;

    assert!(matches!(
        parse_snapshot(json),
        Err(DatasetError::UnsupportedSchemaVersion(99))
    ));
}

#[test]
fn atomic_write_replaces_previous_snapshot() {
    let dir = std::env::temp_dir().join("rumble-ai-clearance-test-replace");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("snapshot.json");

    let first = Snapshot::new(sample_manifest(), vec![sample_entry("acme/v1")]).expect("valid");
    write_snapshot_atomic(&path, &first).expect("first write");

    let second = Snapshot::new(sample_manifest(), vec![sample_entry("acme/v2")]).expect("valid");
    write_snapshot_atomic(&path, &second).expect("second write");

    let loaded = load_snapshot(&path).expect("loads");
    assert_eq!(loaded.models()[0].id(), "acme/v2");

    std::fs::remove_dir_all(&dir).ok();
}
