//! `clearance` — sync, validate, evaluate, explain and gate (`check`) AI model
//! eligibility against a versioned policy. Fail-closed everywhere: any
//! invalid input aborts with exit code 2; `check` exits 1 on a non-eligible
//! model so CI pipelines can gate on it.
#![forbid(unsafe_code)]

mod commands;
mod report;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "clearance",
    version,
    about = "Security clearance for AI models"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Inputs shared by every evaluation-flavoured command.
#[derive(Args)]
struct EvalInputs {
    /// Default rulebook YAML (shipped in content/rulebook/).
    #[arg(long)]
    rulebook: PathBuf,
    /// Organisation policy YAML.
    #[arg(long)]
    policy: PathBuf,
    /// Snapshot JSON produced by `clearance sync`.
    #[arg(long)]
    snapshot: PathBuf,
    /// Need profile YAML (task, purpose, sensitivity).
    #[arg(long)]
    need: PathBuf,
}

#[derive(Subcommand)]
enum Command {
    /// Validate policy files (and optionally a snapshot) without evaluating.
    Validate {
        #[arg(long)]
        rulebook: PathBuf,
        #[arg(long)]
        policy: PathBuf,
        #[arg(long)]
        governance: Option<PathBuf>,
        #[arg(long)]
        snapshot: Option<PathBuf>,
        #[arg(long)]
        need: Option<PathBuf>,
    },
    /// Build the org-local snapshot (never committed: AA terms are
    /// internal-use-only). Offline mode via --aa-file/--hf-file; live mode
    /// reads the AA key from the AA_API_KEY environment variable.
    Sync {
        #[arg(long)]
        governance: PathBuf,
        #[arg(long)]
        out: PathBuf,
        /// Recorded AA response (offline/air-gapped mode).
        #[arg(long)]
        aa_file: Option<PathBuf>,
        /// Recorded HF response (offline/air-gapped mode).
        #[arg(long)]
        hf_file: Option<PathBuf>,
        /// Override the generation timestamp (reproducible builds).
        #[arg(long)]
        generated_at: Option<String>,
    },
    /// List eligible models for a need, ranked; never mixes in refused ones.
    Evaluate {
        #[command(flatten)]
        inputs: EvalInputs,
        #[arg(long)]
        json: bool,
    },
    /// Rule-by-rule verdict for one model.
    Explain {
        model: String,
        #[command(flatten)]
        inputs: EvalInputs,
        #[arg(long)]
        json: bool,
    },
    /// CI gate: exit 0 if the model is eligible, 1 otherwise.
    Check {
        model: String,
        #[command(flatten)]
        inputs: EvalInputs,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let outcome = match cli.command {
        Command::Validate {
            rulebook,
            policy,
            governance,
            snapshot,
            need,
        } => commands::validate(
            &rulebook,
            &policy,
            governance.as_deref(),
            snapshot.as_deref(),
            need.as_deref(),
        ),
        Command::Sync {
            governance,
            out,
            aa_file,
            hf_file,
            generated_at,
        } => commands::sync(
            &governance,
            &out,
            aa_file.as_deref(),
            hf_file.as_deref(),
            generated_at,
        ),
        Command::Evaluate { inputs, json } => commands::evaluate(&inputs, json),
        Command::Explain {
            model,
            inputs,
            json,
        } => commands::explain(&model, &inputs, json),
        Command::Check { model, inputs } => commands::check(&model, &inputs),
    };

    match outcome {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(2)
        }
    }
}
