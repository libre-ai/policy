//! Merge the three sources into one validated snapshot.
//!
//! Authority rules:
//! - Origin comes from the curated governance dataset, never from AA's own
//!   creator country (kept as corroboration only).
//! - Openness derives from the HF licence tag; no licence → unknown
//!   (fail-closed downstream).
//! - API-path jurisdiction is approximated by the provider's HQ country
//!   (v1); unresolved providers leave it unknown — deny-by-default applies.

use rumble_ai_clearance_dataset::{
    Governance, Manifest, Provenance, Snapshot, SnapshotEntry, SourceInfo, SourceKind,
};
use rumble_ai_clearance_domain::{
    ApiKind, BenchDimension, CountryCode, Hosting, Model, Openness, Price,
};

use crate::aa::AaModel;
use crate::error::SyncError;
use crate::hf::HfModel;

/// Timestamps injected by the caller — the pipeline never reads the clock
/// itself, which keeps builds reproducible.
#[derive(Debug, Clone)]
pub struct SyncTimestamps {
    pub generated_at: String,
    pub aa_fetched_at: Option<String>,
    pub hf_fetched_at: Option<String>,
    pub curated_version: String,
}

const AA_SOURCE_ID: &str = "artificial-analysis";
const HF_SOURCE_ID: &str = "hugging-face";

/// OSI-approved licences → open_source; other published-weights licences
/// (llama, gemma, openrail…) → open_weight.
const OSI_LICENCES: &[&str] = &[
    "apache-2.0",
    "mit",
    "bsd-2-clause",
    "bsd-3-clause",
    "mpl-2.0",
    "gpl-2.0",
    "gpl-3.0",
    "lgpl-2.1",
    "lgpl-3.0",
];

fn classify_licence(license: Option<&str>) -> Option<Openness> {
    license.map(|id| {
        if OSI_LICENCES.contains(&id.to_ascii_lowercase().as_str()) {
            Openness::OpenSource
        } else {
            Openness::OpenWeight
        }
    })
}

/// Lowercased alphanumeric form used to match AA slugs to HF repo names.
fn normalize(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Kebab-case fallback id for creators absent from the curated dataset.
fn slugify(name: &str) -> String {
    let mut slug = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    slug.trim_end_matches('-').to_string()
}

/// Build a validated snapshot from parsed AA models, HF models and the
/// curated governance dataset.
pub fn build_snapshot(
    governance: &Governance,
    aa_models: &[AaModel],
    hf_models: &[HfModel],
    timestamps: SyncTimestamps,
) -> Result<Snapshot, SyncError> {
    let mut sources = Vec::new();
    if let Some(fetched_at) = &timestamps.aa_fetched_at {
        sources.push(SourceInfo::new(
            AA_SOURCE_ID,
            SourceKind::ArtificialAnalysis,
            fetched_at.clone(),
        ));
    }
    if let Some(fetched_at) = &timestamps.hf_fetched_at {
        sources.push(SourceInfo::new(
            HF_SOURCE_ID,
            SourceKind::HuggingFace,
            fetched_at.clone(),
        ));
    }
    sources.push(SourceInfo::new(
        timestamps.curated_version.clone(),
        SourceKind::Curated,
        timestamps.generated_at.clone(),
    ));

    let curated_id = timestamps.curated_version.clone();
    let mut entries = Vec::new();
    let mut matched_hf: Vec<&str> = Vec::new();

    for aa_model in aa_models {
        // Curated governance is the authority for provider identity/origin.
        let provider = governance.providers().iter().find(|provider| {
            provider
                .name()
                .eq_ignore_ascii_case(aa_model.creator_name())
                || provider.id() == slugify(aa_model.creator_name())
        });
        let provider_id = provider
            .map(|p| p.id().to_string())
            .unwrap_or_else(|| slugify(aa_model.creator_name()));
        let origin = provider.map(|p| CountryCode::new(p.origin()));

        let mut model = Model::new(format!("{provider_id}/{}", aa_model.slug()), &provider_id);
        let mut provenance = Provenance::new(AA_SOURCE_ID);

        if let Some(origin) = origin.clone() {
            model = model.with_origin(origin);
            provenance = provenance.with_governance(curated_id.clone());
        }

        // Provider API path; jurisdiction approximated by provider HQ (v1).
        model = model.with_hosting(Hosting::Api {
            kind: ApiKind::Provider,
            country: None,
            jurisdiction: origin,
        });
        provenance = provenance.with_hosting(curated_id.clone());

        let mut has_bench = false;
        for dimension in [
            BenchDimension::Intelligence,
            BenchDimension::Coding,
            BenchDimension::Agentic,
            BenchDimension::Math,
            BenchDimension::Multilingual,
        ] {
            if let Some(score) = aa_model.index(dimension) {
                model = model.with_bench(dimension, score);
                has_bench = true;
            }
        }
        if has_bench {
            provenance = provenance.with_bench(AA_SOURCE_ID);
        }
        if let (Some(input), Some(output)) = (aa_model.price_input(), aa_model.price_output()) {
            model = model.with_price(Price::per_mtok(input, output));
        }

        // Published weights on HF? Same provider + matching name → merge.
        let hf_match = hf_models.iter().find(|hf| {
            let same_provider = governance
                .provider_for_alias(hf.org())
                .is_some_and(|p| p.id() == provider_id);
            same_provider && normalize(hf.repo_name()) == normalize(aa_model.slug())
        });
        if let Some(hf_model) = hf_match {
            matched_hf.push(hf_model.repo_id());
            model = model
                .with_self_hostable(true)
                .with_hosting(Hosting::SelfHosted);
            if let Some(openness) = classify_licence(hf_model.license()) {
                model = model.with_openness(openness);
            }
        } else {
            // AA-only model: closed unless weights show up elsewhere.
            model = model
                .with_openness(Openness::Closed)
                .with_self_hostable(false);
        }

        entries.push(SnapshotEntry::new(model, provenance));
    }

    // HF-only models: open weights without AA coverage (bench stays unknown).
    for hf_model in hf_models {
        if matched_hf.contains(&hf_model.repo_id()) {
            continue;
        }
        let provider = governance.provider_for_alias(hf_model.org());
        let provider_id = provider
            .map(|p| p.id().to_string())
            .unwrap_or_else(|| hf_model.org().to_string());

        let mut model = Model::new(
            format!(
                "{provider_id}/{}",
                hf_model.repo_name().to_ascii_lowercase()
            ),
            &provider_id,
        )
        .with_self_hostable(true)
        .with_hosting(Hosting::SelfHosted);
        let mut provenance = Provenance::new(HF_SOURCE_ID);

        if let Some(provider) = provider {
            model = model.with_origin(CountryCode::new(provider.origin()));
            provenance = provenance.with_governance(curated_id.clone());
        }
        if let Some(openness) = classify_licence(hf_model.license()) {
            model = model.with_openness(openness);
        }

        entries.push(SnapshotEntry::new(model, provenance));
    }

    let manifest = Manifest::new(timestamps.generated_at, sources);
    Ok(Snapshot::new(manifest, entries)?)
}
