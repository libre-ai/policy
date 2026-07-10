//! `clearance-api` — serve the org's snapshot and effective policy over a
//! read-only HTTP API. Refuses to boot on any invalid input.
#![forbid(unsafe_code)]

use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(name = "clearance-api", version, about = "Read-only clearance API")]
struct Args {
    #[arg(long)]
    rulebook: PathBuf,
    #[arg(long)]
    policy: PathBuf,
    #[arg(long)]
    snapshot: PathBuf,
    /// Bind address. Loopback by default: network exposure is the org's
    /// deliberate choice, never ours.
    #[arg(long, default_value = "127.0.0.1:8080")]
    addr: String,
    /// Opt-in CORS for exactly one origin (web UI server mode on another
    /// host). No header is sent when unset.
    #[arg(long)]
    cors_allow_origin: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let state = rumble_ai_clearance_api::load_state(&args.rulebook, &args.policy, &args.snapshot)?;
    let router =
        rumble_ai_clearance_api::build_router_with_cors(state, args.cors_allow_origin.as_deref())?;

    let listener = tokio::net::TcpListener::bind(&args.addr).await?;
    println!("clearance-api listening on {}", args.addr);
    axum::serve(listener, router).await?;
    Ok(())
}
