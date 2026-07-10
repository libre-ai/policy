//! Read-only axum API: no HTTP mutation by construction — policy and snapshot
//! only change via files + redeploy, so there is nothing to protect in
//! writing and no application-level authz in v1 (network exposure is the
//! org's call). Refuses to boot on invalid inputs. Envelope `{ data, meta }`,
//! cursor pagination, no PII anywhere (the API never logs request bodies).
#![forbid(unsafe_code)]

use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::extract::{Json, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use serde::Deserialize;

use rumble_ai_clearance_dataset::{Snapshot, load_snapshot};
use rumble_ai_clearance_domain::{Policy, Verdict, evaluate as engine_evaluate, rank, verdict_for};
use rumble_ai_clearance_policy::{
    NeedInput, RankingConfig, compile, compile_ranking, parse_need, parse_policy,
};

/// Everything the handlers read. Immutable after boot.
#[derive(Clone)]
pub struct AppState {
    snapshot: Arc<Snapshot>,
    policy: Arc<Policy>,
    ranking: Arc<RankingConfig>,
}

impl AppState {
    pub fn new(snapshot: Snapshot, policy: Policy, ranking: RankingConfig) -> Self {
        Self {
            snapshot: Arc::new(snapshot),
            policy: Arc::new(policy),
            ranking: Arc::new(ranking),
        }
    }
}

/// Load state from files. Fail-closed: any invalid input refuses to boot.
pub fn load_state(rulebook: &Path, policy: &Path, snapshot: &Path) -> anyhow::Result<AppState> {
    let read = |path: &Path| -> anyhow::Result<String> {
        std::fs::read_to_string(path)
            .map_err(|err| anyhow::anyhow!("reading {}: {err}", path.display()))
    };
    let rulebook_doc = parse_policy(&read(rulebook)?)?;
    let org_doc = parse_policy(&read(policy)?)?;
    let effective = compile(&rulebook_doc, &org_doc)?;
    let ranking = compile_ranking(&rulebook_doc, &org_doc);
    let snapshot = load_snapshot(snapshot)?;
    Ok(AppState::new(snapshot, effective, ranking))
}

/// The whole surface: five read-only routes. No CORS header by default —
/// same-origin or reverse-proxy deployments need none.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/dataset", get(dataset))
        .route("/api/v1/models", get(models))
        .route("/api/v1/evaluations", post(evaluations))
        .route("/api/v1/verdicts", get(verdicts))
        .route("/api/v1/policy", get(policy_view))
        .with_state(state)
}

/// Same router with an opt-in CORS layer scoped to exactly one origin (for
/// the web UI's server mode when it is served from another host). Never a
/// wildcard: cross-origin exposure is a deliberate, named choice.
pub fn build_router_with_cors(
    state: AppState,
    cors_allow_origin: Option<&str>,
) -> anyhow::Result<Router> {
    let router = build_router(state);
    match cors_allow_origin {
        None => Ok(router),
        Some(origin) => {
            let origin: axum::http::HeaderValue = origin
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid CORS origin: {origin}"))?;
            let cors = tower_http::cors::CorsLayer::new()
                .allow_origin(origin)
                .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
                .allow_headers([axum::http::header::CONTENT_TYPE]);
            Ok(router.layer(cors))
        }
    }
}

fn envelope(data: serde_json::Value, meta: serde_json::Value) -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "data": data, "meta": meta }))
}

async fn dataset(State(state): State<AppState>) -> impl IntoResponse {
    let manifest = state.snapshot.manifest();
    envelope(
        serde_json::json!({
            "generated_at": manifest.generated_at(),
            "sources": manifest.sources(),
        }),
        serde_json::json!({}),
    )
}

#[derive(Debug, Deserialize)]
struct Page {
    #[serde(default)]
    cursor: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

const MAX_PAGE: usize = 200;
const DEFAULT_PAGE: usize = 50;

async fn models(State(state): State<AppState>, Query(page): Query<Page>) -> impl IntoResponse {
    let limit = page.limit.unwrap_or(DEFAULT_PAGE).min(MAX_PAGE);

    let mut all = state.snapshot.models();
    all.sort_by(|a, b| a.id().cmp(b.id()));

    let after = page.cursor.unwrap_or_default();
    let remaining: Vec<_> = all
        .into_iter()
        .filter(|model| model.id() > after.as_str())
        .collect();
    let has_more = remaining.len() > limit;
    let page_items: Vec<_> = remaining.into_iter().take(limit).collect();
    let next_cursor = if has_more {
        page_items
            .last()
            .map(|model| serde_json::Value::String(model.id().to_string()))
            .unwrap_or(serde_json::Value::Null)
    } else {
        serde_json::Value::Null
    };

    // Serialization of Model cannot realistically fail, but hiding an error
    // behind an empty page would be a silent degradation: fail loudly.
    let data = match serde_json::to_value(&page_items) {
        Ok(data) => data,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({ "error": err.to_string() })),
            )
                .into_response();
        }
    };
    envelope(
        data,
        serde_json::json!({
            "next_cursor": next_cursor,
            "snapshot_generated_at": state.snapshot.manifest().generated_at(),
        }),
    )
    .into_response()
}

async fn evaluations(
    State(state): State<AppState>,
    Json(need_input): Json<NeedInput>,
) -> impl IntoResponse {
    let need = need_input.profile();
    let models = state.snapshot.models();

    let mut eligible = Vec::new();
    let mut ineligible_count = 0usize;
    let mut indeterminate_count = 0usize;
    for model in &models {
        match engine_evaluate(model, &state.policy, &need) {
            Verdict::Eligible { viable_hostings } => eligible.push((model, viable_hostings)),
            Verdict::Ineligible { .. } => ineligible_count += 1,
            Verdict::Indeterminate { .. } => indeterminate_count += 1,
        }
    }

    let spec = state.ranking.spec_for(need.task());
    let eligible_models: Vec<_> = eligible.iter().map(|(model, _)| *model).collect();
    let ranked = rank(&eligible_models, &spec);
    let entries: Vec<_> = ranked
        .iter()
        .map(|model| {
            let hostings = eligible
                .iter()
                .find(|(candidate, _)| candidate.id() == model.id())
                .map(|(_, hostings)| hostings.clone())
                .unwrap_or_default();
            serde_json::json!({ "model": model.id(), "viable_hostings": hostings })
        })
        .collect();

    envelope(
        serde_json::json!({
            "eligible": entries,
            "ineligible_count": ineligible_count,
            "indeterminate_count": indeterminate_count,
        }),
        serde_json::json!({
            "snapshot_generated_at": state.snapshot.manifest().generated_at(),
        }),
    )
}

#[derive(Debug, Deserialize)]
struct VerdictQuery {
    model: String,
    task: String,
    purpose: String,
    sensitivity: String,
}

async fn verdicts(
    State(state): State<AppState>,
    Query(query): Query<VerdictQuery>,
) -> axum::response::Response {
    // Route the query values through the same strict parser as need files:
    // anything malformed is refused, never guessed.
    let yaml = format!(
        "task: {}\npurpose: {}\nsensitivity: {}\n",
        query.task, query.purpose, query.sensitivity
    );
    let need = match parse_need(&yaml) {
        Ok(need) => need,
        Err(err) => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                axum::Json(serde_json::json!({ "error": err.to_string() })),
            )
                .into_response();
        }
    };

    let verdict = verdict_for(&state.snapshot.models(), &query.model, &state.policy, &need);
    envelope(
        serde_json::json!({ "model": query.model, "verdict": verdict }),
        serde_json::json!({
            "snapshot_generated_at": state.snapshot.manifest().generated_at(),
        }),
    )
    .into_response()
}

async fn policy_view(State(state): State<AppState>) -> impl IntoResponse {
    envelope(
        serde_json::to_value(state.policy.as_ref()).unwrap_or_default(),
        serde_json::json!({}),
    )
}
