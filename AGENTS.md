# Model Policy Canonical Agent Rules

## Purpose

Model Policy checks whether a business need may use a given AI model against
an organization's security policy, producing explainable, replayable
verdicts instead of opaque model scoring — the couche-1 product for model
governance in the constellation.
Doctrine lives upstream: https://raw.githubusercontent.com/libre-ai/governance/main/docs/README.md

## Domain doctrine

- No opaque scoring, no evaluation of models outside the declared policy —
  verdicts must be explainable and replayable against the reference
  implementation (`policy-core-ref`).
- Evaluation is deterministic and deny-by-default over immutable model
  snapshots: zero network calls during evaluation.
- Canonical surfaces are `apps/model-policy` (application) and
  `crates/policy-core` (WIT-vendored reference frontier).
- Bricks and contracts this repo depends on (`web-platform`, `contracts`,
  `governance`) are consumed pinned, never redefined here.

## Commands

- `bun install` — installs the Bun workspace and its pinned git-deps.
- `bun run test` (`cargo test --locked`) — Rust workspace tests.
- `bun run lint` — Biome across the TypeScript surface.
- `bun run check` — aggregate gate (toolchain, WIT, app tests, secret scan,
  personal-data boundary, lint); run before pushing.

## Working here

- Security > quality > performance > completeness, in that order on conflict.
- Check real state before editing: `git status --short` and `bun run test`.
- English for code, comments and this file; French stays the human
  conversation language elsewhere.
- Never commit a machine-local absolute filesystem path; use repo-relative
  paths or `~` instead.
