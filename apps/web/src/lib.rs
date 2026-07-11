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
                Some(country) => format!("{kind} ({country})"),
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
            .chain(violations.iter().map(|rule| format!("violates: {rule}")))
            .collect(),
        Verdict::Indeterminate { missing } => {
            std::iter::once("INDETERMINATE (missing data, fail-closed)".to_string())
                .chain(
                    missing
                        .iter()
                        .map(|(rule, dim)| format!("missing: {dim:?} (required by {rule})")),
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

/// Normalize an API base URL before issuing a server-mode request.
pub fn normalize_server_base_url(base_url: &str) -> Result<String, String> {
    let normalized = base_url.trim().trim_end_matches('/');
    if normalized.is_empty() {
        return Err("API base URL is required in server mode".to_string());
    }
    if !normalized.starts_with("http://") && !normalized.starts_with("https://") {
        return Err("API base URL must start with http:// or https://".to_string());
    }
    Ok(normalized.to_string())
}

/// Parse the API envelope strictly. Missing or mistyped fields fail closed
/// instead of being silently coerced to empty lists or zero counters.
pub fn parse_remote_outcome(value: &serde_json::Value) -> Result<Outcome, String> {
    let data = value
        .get("data")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| "API response is missing the data object".to_string())?;
    let entries = data
        .get("eligible")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "API response has no eligible array".to_string())?;

    let eligible = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let model = entry
                .get("model")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| format!("eligible[{index}] has no model string"))?;
            let hostings: Vec<Hosting> = serde_json::from_value(
                entry
                    .get("viable_hostings")
                    .cloned()
                    .ok_or_else(|| format!("eligible[{index}] has no viable_hostings"))?,
            )
            .map_err(|error| format!("eligible[{index}] has invalid viable_hostings: {error}"))?;
            Ok(EligibleRow {
                model: model.to_string(),
                hostings: hostings.iter().map(hosting_label).collect(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let count = |name: &str| -> Result<usize, String> {
        let value = data
            .get(name)
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| format!("API response has no valid {name}"))?;
        usize::try_from(value).map_err(|_| format!("API response {name} exceeds this platform"))
    };
    let snapshot_version = value
        .get("meta")
        .and_then(|meta| meta.get("snapshot_generated_at"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "API response has no snapshot_generated_at".to_string())?;

    Ok(Outcome {
        eligible,
        ineligible_count: count("ineligible_count")?,
        indeterminate_count: count("indeterminate_count")?,
        snapshot_version: snapshot_version.to_string(),
    })
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
    let base_url = normalize_server_base_url(base_url)?;
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
    parse_remote_outcome(&value)
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

const FONTS_CSS: Asset = asset!(
    "/assets/fonts/fonts.css",
    AssetOptions::css().with_hash_suffix(false)
);
#[used]
static INTER_FONT: Asset = asset!(
    "/assets/fonts/inter-latin-wght-normal.woff2",
    AssetOptions::builder().with_hash_suffix(false)
);
#[used]
static DISPLAY_FONT: Asset = asset!(
    "/assets/fonts/plus-jakarta-sans-latin-wght-normal.woff2",
    AssetOptions::builder().with_hash_suffix(false)
);
const TOKENS_CSS: Asset = asset!("/assets/libre-ia/tokens.css");
const THEMES_CSS: Asset = asset!("/assets/libre-ia/themes.css");
const COMPONENTS_CSS: Asset = asset!("/assets/libre-ia/components.css");
const CLEARANCE_CSS: Asset = asset!("/assets/clearance.css");
const FAVICON: Asset = asset!("/assets/favicon.svg");

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
        document::Link { rel: "icon", r#type: "image/svg+xml", href: FAVICON }
        document::Link { rel: "stylesheet", href: FONTS_CSS }
        document::Link { rel: "stylesheet", href: TOKENS_CSS }
        document::Link { rel: "stylesheet", href: THEMES_CSS }
        document::Link { rel: "stylesheet", href: COMPONENTS_CSS }
        document::Link { rel: "stylesheet", href: CLEARANCE_CSS }
        main { class: "clearance-app",
            header {
                p { class: "clearance-kicker", "rumble-ai-clearance · Libre IA" }
                h1 { "AI Clearance" }
                p {
                "Security clearance for AI models: match a business need against "
                    "your organisation's policy and get explainable, rule-by-rule verdicts."
                }
            }

            fieldset { class: "lia-card clearance-panel",
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
                            class: "lia-input",
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
                            class: "lia-input",
                            value: snapshot_json(),
                            oninput: move |evt| snapshot_json.set(evt.value()),
                        }
                    }
                    details {
                        summary { "Org policy YAML (default: no US/CN data flow, self-host OK)" }
                        textarea {
                            class: "lia-input",
                            value: policy_yaml(),
                            oninput: move |evt| policy_yaml.set(evt.value()),
                        }
                    }
                }
            }

            fieldset { class: "lia-card clearance-panel",
                legend { "Business need" }
                div { class: "clearance-selects",
                select {
                    class: "lia-input clearance-select",
                    aria_label: "task",
                    onchange: move |evt| task.set(evt.value()),
                    for option in TASKS {
                        option { value: option, selected: task() == option, {option} }
                    }
                }
                select {
                    class: "lia-input clearance-select",
                    aria_label: "purpose",
                    onchange: move |evt| purpose.set(evt.value()),
                    for option in PURPOSES {
                        option { value: option, selected: purpose() == option, {option} }
                    }
                }
                select {
                    class: "lia-input clearance-select",
                    aria_label: "sensitivity",
                    onchange: move |evt| sensitivity.set(evt.value()),
                    for option in SENSITIVITIES {
                        option { value: option, selected: sensitivity() == option, {option} }
                    }
                }
                button { class: "lia-button lia-button--primary", onclick: run, "Evaluate" }
                }
            }

            if !error().is_empty() {
                p { class: "lia-alert clearance-error", role: "alert", "{error}" }
            }

            if let Some(result) = outcome() {
                section { class: "clearance-result",
                    h2 { "Eligible models" }
                    p { class: "counts",
                        "eligible: {result.eligible.len()} · ineligible: {result.ineligible_count} · "
                        "indeterminate (fail-closed): {result.indeterminate_count} · "
                        "snapshot: {result.snapshot_version}"
                    }
                    div { class: "clearance-table-wrap",
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
                                        // Local mode only: explaining against
                                        // the local snapshot while the list
                                        // came from the API would lie.
                                        if !server_mode() {
                                        button {
                                            class: "lia-button",
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
