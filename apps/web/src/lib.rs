//! Dioxus web UI, dual mode.
//!
//! - **Local mode** (default, also the public demo): the same engine as the
//!   CLI compiled to WASM — snapshot and policy are pasted or loaded in the
//!   browser and never leave it.
//! - **Server mode**: thin client of the org's read-only `clearance-api`
//!   (wasm-only fetch; the host/SSR build stubs it out).
//!
//! The public demo ships an illustrative catalogue without any Artificial
//! Analysis data (their free tier is internal-use-only): benchmark columns
//! stay empty until the org syncs with its own key.
#![forbid(unsafe_code)]

use dioxus::prelude::*;

use rumble_ai_clearance_dataset::{Snapshot, parse_snapshot};
use rumble_ai_clearance_domain::{
    ApiKind, Hosting, Verdict, evaluate as engine_evaluate, rank, verdict_for,
};
use rumble_ai_clearance_policy::{compile, compile_ranking, parse_need, parse_policy};

/// Illustrative demo catalogue (no AA data by design).
pub const DEMO_SNAPSHOT: &str = include_str!("../assets/demo-snapshot.json");
/// The shipped default rulebook (sourced rules).
pub const DEFAULT_RULEBOOK: &str = include_str!("../../../content/rulebook/rulebook.yaml");
/// The canonical example org policy: no US/CN data flow, self-host OK.
pub const EXAMPLE_ORG_POLICY: &str =
    include_str!("../../../examples/policy-no-us-cn-selfhost-ok.yaml");

/// Outcome of one evaluation, mode-independent.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub eligible: Vec<EligibleRow>,
    pub ineligible_count: usize,
    pub indeterminate_count: usize,
    pub snapshot_version: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EligibleRow {
    pub model: String,
    pub hostings: Vec<String>,
}

/// Compact label for a deployment path.
pub fn hosting_label(hosting: &Hosting) -> String {
    match hosting {
        Hosting::SelfHosted => "self-hosted".to_string(),
        Hosting::Api {
            kind, jurisdiction, ..
        } => {
            let kind = match kind {
                ApiKind::EuSovereign => "eu-sovereign-api",
                ApiKind::Provider => "provider-api",
                ApiKind::Hyperscaler => "hyperscaler-api",
            };
            match jurisdiction {
                Some(country) => format!("{kind} ({country:?})"),
                None => format!("{kind} (jurisdiction unknown)"),
            }
        }
    }
}

/// Rule-by-rule text lines for a verdict.
pub fn verdict_lines(verdict: &Verdict) -> Vec<String> {
    match verdict {
        Verdict::Eligible { viable_hostings } => std::iter::once("ELIGIBLE".to_string())
            .chain(
                viable_hostings
                    .iter()
                    .map(|hosting| format!("viable: {}", hosting_label(hosting))),
            )
            .collect(),
        Verdict::Ineligible { violations } => std::iter::once("INELIGIBLE".to_string())
            .chain(violations.iter().map(|rule| format!("violates: {rule:?}")))
            .collect(),
        Verdict::Indeterminate { missing } => {
            std::iter::once("INDETERMINATE (missing data, fail-closed)".to_string())
                .chain(
                    missing
                        .iter()
                        .map(|(rule, dim)| format!("missing: {dim:?} (required by {rule:?})")),
                )
                .collect()
        }
    }
}

fn need_yaml(task: &str, purpose: &str, sensitivity: &str) -> String {
    format!("task: {task}\npurpose: {purpose}\nsensitivity: {sensitivity}\n")
}

/// Local-mode evaluation: everything happens in this process (browser WASM
/// or host tests) — nothing leaves it. Pure and host-testable.
pub fn evaluate_locally(
    snapshot_json: &str,
    org_policy_yaml: &str,
    task: &str,
    purpose: &str,
    sensitivity: &str,
) -> Result<Outcome, String> {
    let snapshot: Snapshot = parse_snapshot(snapshot_json).map_err(|e| e.to_string())?;
    let rulebook = parse_policy(DEFAULT_RULEBOOK).map_err(|e| e.to_string())?;
    let org = parse_policy(org_policy_yaml).map_err(|e| e.to_string())?;
    let policy = compile(&rulebook, &org).map_err(|e| e.to_string())?;
    let ranking = compile_ranking(&rulebook, &org);
    let need = parse_need(&need_yaml(task, purpose, sensitivity)).map_err(|e| e.to_string())?;

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
    let rows = ranked
        .iter()
        .map(|model| {
            let hostings = eligible
                .iter()
                .find(|(candidate, _)| candidate.id() == model.id())
                .map(|(_, hostings)| hostings.iter().map(hosting_label).collect())
                .unwrap_or_default();
            EligibleRow {
                model: model.id().to_string(),
                hostings,
            }
        })
        .collect();

    Ok(Outcome {
        eligible: rows,
        ineligible_count,
        indeterminate_count,
        snapshot_version: snapshot.manifest().generated_at().to_string(),
    })
}

/// Local-mode verdict for one model (deny-by-default on unknown ids).
pub fn explain_locally(
    snapshot_json: &str,
    org_policy_yaml: &str,
    task: &str,
    purpose: &str,
    sensitivity: &str,
    model: &str,
) -> Result<Vec<String>, String> {
    let snapshot: Snapshot = parse_snapshot(snapshot_json).map_err(|e| e.to_string())?;
    let rulebook = parse_policy(DEFAULT_RULEBOOK).map_err(|e| e.to_string())?;
    let org = parse_policy(org_policy_yaml).map_err(|e| e.to_string())?;
    let policy = compile(&rulebook, &org).map_err(|e| e.to_string())?;
    let need = parse_need(&need_yaml(task, purpose, sensitivity)).map_err(|e| e.to_string())?;
    let verdict = verdict_for(&snapshot.models(), model, &policy, &need);
    Ok(verdict_lines(&verdict))
}

/// Server-mode evaluation: POST /api/v1/evaluations on the org's API.
/// Browser-only; the host/SSR build degrades with an explicit error.
#[cfg(target_arch = "wasm32")]
pub async fn evaluate_remotely(
    base_url: &str,
    task: &str,
    purpose: &str,
    sensitivity: &str,
) -> Result<Outcome, String> {
    let body = serde_json::json!({
        "task": task,
        "purpose": purpose,
        "sensitivity": sensitivity,
    });
    let response = gloo_net::http::Request::post(&format!("{base_url}/api/v1/evaluations"))
        .header("content-type", "application/json")
        .body(body.to_string())
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.ok() {
        return Err(format!("API error: HTTP {}", response.status()));
    }
    let value: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    Ok(Outcome {
        eligible: value["data"]["eligible"]
            .as_array()
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(|entry| entry["model"].as_str())
                    .map(|model| EligibleRow {
                        model: model.to_string(),
                        hostings: Vec::new(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        ineligible_count: value["data"]["ineligible_count"].as_u64().unwrap_or(0) as usize,
        indeterminate_count: value["data"]["indeterminate_count"].as_u64().unwrap_or(0) as usize,
        snapshot_version: value["meta"]["snapshot_generated_at"]
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn evaluate_remotely(
    _base_url: &str,
    _task: &str,
    _purpose: &str,
    _sensitivity: &str,
) -> Result<Outcome, String> {
    Err("server mode is only available in the browser build".to_string())
}

const TASKS: [&str; 8] = [
    "code_generation",
    "agentic",
    "summary_extraction",
    "classification",
    "writing",
    "translation",
    "reasoning",
    "general_chat",
];
const PURPOSES: [&str; 4] = [
    "public_content",
    "personal_data",
    "automated_decision",
    "health_data",
];
const SENSITIVITIES: [&str; 4] = ["c0", "c1", "c2", "c3"];

const STYLE: &str = r#"
:root { color-scheme: light dark; font-family: system-ui, sans-serif; }
body { margin: 0; }
main { max-width: 70rem; margin: 0 auto; padding: 1.5rem 1rem; }
h1 { font-size: 1.5rem; }
textarea { width: 100%; min-height: 10rem; font-family: monospace; font-size: 0.85rem; }
table { width: 100%; border-collapse: collapse; margin-top: 1rem; }
th, td { text-align: left; padding: 0.5rem 0.75rem; border-bottom: 1px solid color-mix(in srgb, currentColor 25%, transparent); }
select { margin-right: 1rem; padding: 0.25rem; }
button { padding: 0.5rem 1rem; cursor: pointer; }
.error { color: #c0392b; }
.banner { font-size: 0.85rem; opacity: 0.8; margin-top: 2rem; }
.counts { margin-top: 0.75rem; font-size: 0.9rem; }
details { margin-top: 0.5rem; }
fieldset { border: 1px solid color-mix(in srgb, currentColor 25%, transparent); margin-top: 1rem; }
"#;

/// The application shell (SSR-testable on the host).
#[component]
pub fn App() -> Element {
    let mut snapshot_json = use_signal(|| DEMO_SNAPSHOT.to_string());
    let mut policy_yaml = use_signal(|| EXAMPLE_ORG_POLICY.to_string());
    let mut task = use_signal(|| "code_generation".to_string());
    let mut purpose = use_signal(|| "public_content".to_string());
    let mut sensitivity = use_signal(|| "c0".to_string());
    let mut server_mode = use_signal(|| false);
    let mut server_url = use_signal(String::new);
    let mut outcome = use_signal(|| Option::<Outcome>::None);
    let mut error = use_signal(String::new);
    let mut detail = use_signal(Vec::<String>::new);

    let run = move |_| {
        detail.set(Vec::new());
        if server_mode() {
            let base = server_url();
            spawn(async move {
                match evaluate_remotely(&base, &task(), &purpose(), &sensitivity()).await {
                    Ok(result) => {
                        error.set(String::new());
                        outcome.set(Some(result));
                    }
                    Err(message) => {
                        outcome.set(None);
                        error.set(message);
                    }
                }
            });
        } else {
            match evaluate_locally(
                &snapshot_json(),
                &policy_yaml(),
                &task(),
                &purpose(),
                &sensitivity(),
            ) {
                Ok(result) => {
                    error.set(String::new());
                    outcome.set(Some(result));
                }
                Err(message) => {
                    outcome.set(None);
                    error.set(message);
                }
            }
        }
    };

    rsx! {
        style { {STYLE} }
        main {
            h1 { "rumble-ai-clearance" }
            p {
                "Security clearance for AI models: match a business need against "
                "your organisation's policy and get explainable, rule-by-rule verdicts."
            }

            fieldset {
                legend { "Data (local mode: nothing leaves this browser)" }
                label {
                    input {
                        r#type: "checkbox",
                        checked: server_mode(),
                        onchange: move |evt| server_mode.set(evt.checked()),
                    }
                    " Server mode (query your org's clearance-api instead)"
                }
                if server_mode() {
                    p {
                        "API base URL: "
                        input {
                            r#type: "url",
                            placeholder: "http://localhost:8080",
                            value: server_url(),
                            oninput: move |evt| server_url.set(evt.value()),
                        }
                    }
                } else {
                    details {
                        summary { "Snapshot JSON (default: illustrative demo catalogue)" }
                        textarea {
                            value: snapshot_json(),
                            oninput: move |evt| snapshot_json.set(evt.value()),
                        }
                    }
                    details {
                        summary { "Org policy YAML (default: no US/CN data flow, self-host OK)" }
                        textarea {
                            value: policy_yaml(),
                            oninput: move |evt| policy_yaml.set(evt.value()),
                        }
                    }
                }
            }

            fieldset {
                legend { "Business need" }
                select {
                    aria_label: "task",
                    onchange: move |evt| task.set(evt.value()),
                    for option in TASKS {
                        option { value: option, selected: task() == option, {option} }
                    }
                }
                select {
                    aria_label: "purpose",
                    onchange: move |evt| purpose.set(evt.value()),
                    for option in PURPOSES {
                        option { value: option, selected: purpose() == option, {option} }
                    }
                }
                select {
                    aria_label: "sensitivity",
                    onchange: move |evt| sensitivity.set(evt.value()),
                    for option in SENSITIVITIES {
                        option { value: option, selected: sensitivity() == option, {option} }
                    }
                }
                button { onclick: run, "Evaluate" }
            }

            if !error().is_empty() {
                p { class: "error", "{error}" }
            }

            if let Some(result) = outcome() {
                section {
                    h2 { "Eligible models" }
                    p { class: "counts",
                        "eligible: {result.eligible.len()} · ineligible: {result.ineligible_count} · "
                        "indeterminate (fail-closed): {result.indeterminate_count} · "
                        "snapshot: {result.snapshot_version}"
                    }
                    table {
                        thead {
                            tr {
                                th { "rank" }
                                th { "model" }
                                th { "viable hostings" }
                                th { "" }
                            }
                        }
                        tbody {
                            for (index, row) in result.eligible.iter().cloned().enumerate() {
                                tr {
                                    td { "{index + 1}" }
                                    td { "{row.model}" }
                                    td { {row.hostings.join(", ")} }
                                    td {
                                        button {
                                            onclick: {
                                                let model = row.model.clone();
                                                move |_| {
                                                    match explain_locally(
                                                        &snapshot_json(),
                                                        &policy_yaml(),
                                                        &task(),
                                                        &purpose(),
                                                        &sensitivity(),
                                                        &model,
                                                    ) {
                                                        Ok(lines) => detail.set(lines),
                                                        Err(message) => error.set(message),
                                                    }
                                                }
                                            },
                                            "explain"
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if !detail().is_empty() {
                        section {
                            h3 { "Verdict" }
                            for line in detail() {
                                p { "{line}" }
                            }
                        }
                    }
                }
            }

            p { class: "banner",
                "Benchmark columns stay empty until you sync with your own key: "
                "Artificial Analysis data is internal-use-only (no redistribution). "
                "Benchmarks, prices and speed: "
                a { href: "https://artificialanalysis.ai/", "Artificial Analysis" }
                " · Catalogue: "
                a { href: "https://huggingface.co/models", "Hugging Face" }
            }
        }
    }
}
