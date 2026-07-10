//! Command implementations. Fail-closed: every loader refuses invalid input
//! outright; nothing evaluates against a partially valid policy or snapshot.

use std::path::Path;
use std::process::ExitCode;

use anyhow::Context;

use rumble_ai_clearance_dataset::{
    Governance, Snapshot, load_snapshot, parse_governance, write_snapshot_atomic,
};
use rumble_ai_clearance_domain::{
    NeedProfile, Policy, Verdict, evaluate as engine_evaluate, rank, verdict_for,
};
use rumble_ai_clearance_policy::{
    RankingConfig, compile, compile_ranking, parse_need, parse_policy,
};
use rumble_ai_clearance_sync::{
    AA_FREE_MODELS_URL, SyncTimestamps, build_snapshot, hf_models_url, parse_aa_response,
    parse_hf_response,
};

use crate::EvalInputs;
use crate::report;

fn read(path: &Path) -> anyhow::Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
}

/// Load and compile rulebook ⊕ org policy.
fn load_effective(rulebook: &Path, policy: &Path) -> anyhow::Result<(Policy, RankingConfig)> {
    let rulebook_doc = parse_policy(&read(rulebook)?)
        .with_context(|| format!("parsing rulebook {}", rulebook.display()))?;
    let org_doc = parse_policy(&read(policy)?)
        .with_context(|| format!("parsing org policy {}", policy.display()))?;
    let effective = compile(&rulebook_doc, &org_doc).context("compiling effective policy")?;
    let ranking = compile_ranking(&rulebook_doc, &org_doc);
    Ok((effective, ranking))
}

fn load_need(path: &Path) -> anyhow::Result<NeedProfile> {
    parse_need(&read(path)?).with_context(|| format!("parsing need profile {}", path.display()))
}

fn load_snapshot_file(path: &Path) -> anyhow::Result<Snapshot> {
    load_snapshot(path).with_context(|| format!("loading snapshot {}", path.display()))
}

fn load_governance(path: &Path) -> anyhow::Result<Governance> {
    parse_governance(&read(path)?).with_context(|| format!("parsing governance {}", path.display()))
}

pub fn validate(
    rulebook: &Path,
    policy: &Path,
    governance: Option<&Path>,
    snapshot: Option<&Path>,
    need: Option<&Path>,
) -> anyhow::Result<ExitCode> {
    load_effective(rulebook, policy)?;
    println!("policy: OK (rulebook ⊕ org compiles)");
    if let Some(path) = governance {
        load_governance(path)?;
        println!("governance: OK");
    }
    if let Some(path) = snapshot {
        load_snapshot_file(path)?;
        println!("snapshot: OK");
    }
    if let Some(path) = need {
        load_need(path)?;
        println!("need: OK");
    }
    Ok(ExitCode::SUCCESS)
}

fn now_rfc3339() -> anyhow::Result<String> {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .context("formatting current time")
}

fn fetch_aa_live() -> anyhow::Result<String> {
    let key = std::env::var("AA_API_KEY")
        .context("AA_API_KEY is not set (required for live sync; use --aa-file offline)")?;
    let response = ureq::get(AA_FREE_MODELS_URL)
        .set("x-api-key", &key)
        .call()
        .context("fetching Artificial Analysis models")?;
    Ok(response.into_string()?)
}

fn fetch_hf_live(
    governance: &Governance,
) -> anyhow::Result<Vec<rumble_ai_clearance_sync::HfModel>> {
    let mut models = Vec::new();
    for provider in governance.providers() {
        for alias in provider.aliases() {
            let response = ureq::get(&hf_models_url(alias))
                .call()
                .with_context(|| format!("fetching Hugging Face models for {alias}"))?;
            models.extend(parse_hf_response(&response.into_string()?)?);
        }
    }
    Ok(models)
}

pub fn sync(
    governance_path: &Path,
    out: &Path,
    aa_file: Option<&Path>,
    hf_file: Option<&Path>,
    generated_at: Option<String>,
) -> anyhow::Result<ExitCode> {
    let governance = load_governance(governance_path)?;
    let generated_at = match generated_at {
        Some(stamp) => stamp,
        None => now_rfc3339()?,
    };

    let (aa_raw, aa_fetched_at) = match aa_file {
        Some(path) => (read(path)?, generated_at.clone()),
        None => (fetch_aa_live()?, now_rfc3339()?),
    };
    let aa_models = parse_aa_response(&aa_raw).context("parsing AA response")?;

    let (hf_models, hf_fetched_at) = match hf_file {
        Some(path) => (
            parse_hf_response(&read(path)?).context("parsing HF response")?,
            generated_at.clone(),
        ),
        None => (fetch_hf_live(&governance)?, now_rfc3339()?),
    };

    let snapshot = build_snapshot(
        &governance,
        &aa_models,
        &hf_models,
        SyncTimestamps {
            generated_at,
            aa_fetched_at: Some(aa_fetched_at),
            hf_fetched_at: Some(hf_fetched_at),
            curated_version: "curated-v1".to_string(),
        },
    )
    .context("building snapshot")?;

    write_snapshot_atomic(out, &snapshot).context("writing snapshot")?;
    println!(
        "snapshot: {} models -> {} (generated_at {})",
        snapshot.entries().len(),
        out.display(),
        snapshot.manifest().generated_at(),
    );
    println!(
        "reminder: AA data is internal-use-only (no redistribution); keep {} out of version control",
        out.display()
    );
    Ok(ExitCode::SUCCESS)
}

pub fn evaluate_report(inputs: &EvalInputs) -> anyhow::Result<report::EvaluateReport> {
    let (policy, ranking) = load_effective(&inputs.rulebook, &inputs.policy)?;
    let need = load_need(&inputs.need)?;
    let snapshot = load_snapshot_file(&inputs.snapshot)?;
    let models = snapshot.models();

    let mut eligible = Vec::new();
    let mut ineligible_count = 0usize;
    let mut indeterminate_count = 0usize;
    for model in &models {
        match engine_evaluate(model, &policy, &need) {
            Verdict::Eligible { viable_hostings } => eligible.push((model, viable_hostings)),
            Verdict::Ineligible { .. } => ineligible_count += 1,
            Verdict::Indeterminate { .. } => indeterminate_count += 1,
        }
    }

    let spec = ranking.spec_for(need.task());
    let eligible_models: Vec<_> = eligible.iter().map(|(model, _)| *model).collect();
    let ranked = rank(&eligible_models, &spec);

    let entries = ranked
        .iter()
        .map(|model| {
            let hostings = eligible
                .iter()
                .find(|(candidate, _)| candidate.id() == model.id())
                .map(|(_, hostings)| report::hosting_labels(hostings))
                .unwrap_or_default();
            report::EligibleEntry {
                model: model.id().to_string(),
                viable_hostings: hostings,
            }
        })
        .collect();

    Ok(report::EvaluateReport {
        data: report::EvaluateData {
            eligible: entries,
            ineligible_count,
            indeterminate_count,
        },
        meta: report::Meta {
            snapshot_generated_at: snapshot.manifest().generated_at().to_string(),
            source_count: snapshot.manifest().sources().len(),
        },
    })
}

pub fn evaluate(inputs: &EvalInputs, json: bool) -> anyhow::Result<ExitCode> {
    let report = evaluate_report(inputs)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "eligible: {} | ineligible: {} | indeterminate: {} (snapshot {})",
            report.data.eligible.len(),
            report.data.ineligible_count,
            report.data.indeterminate_count,
            report.meta.snapshot_generated_at,
        );
        for entry in &report.data.eligible {
            println!("  {}  [{}]", entry.model, entry.viable_hostings.join(", "));
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn model_verdict(model: &str, inputs: &EvalInputs) -> anyhow::Result<(Verdict, String)> {
    let (policy, _) = load_effective(&inputs.rulebook, &inputs.policy)?;
    let need = load_need(&inputs.need)?;
    let snapshot = load_snapshot_file(&inputs.snapshot)?;
    let verdict = verdict_for(&snapshot.models(), model, &policy, &need);
    Ok((verdict, snapshot.manifest().generated_at().to_string()))
}

pub fn explain(model: &str, inputs: &EvalInputs, json: bool) -> anyhow::Result<ExitCode> {
    let (verdict, snapshot_version) = model_verdict(model, inputs)?;
    if json {
        let payload = serde_json::json!({
            "data": { "model": model, "verdict": verdict },
            "meta": { "snapshot_generated_at": snapshot_version },
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("{model} (snapshot {snapshot_version})");
        println!("{}", report::verdict_lines(&verdict).join("\n"));
    }
    Ok(ExitCode::SUCCESS)
}

pub fn check(model: &str, inputs: &EvalInputs) -> anyhow::Result<ExitCode> {
    let (verdict, snapshot_version) = model_verdict(model, inputs)?;
    match verdict {
        Verdict::Eligible { .. } => {
            println!("{model}: ELIGIBLE (snapshot {snapshot_version})");
            Ok(ExitCode::SUCCESS)
        }
        other => {
            println!("{model}: NOT ELIGIBLE (snapshot {snapshot_version})");
            println!("{}", report::verdict_lines(&other).join("\n"));
            Ok(ExitCode::from(1))
        }
    }
}
