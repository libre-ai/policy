# AGENTS.md

Canonical agent-context surface for this repository. `CLAUDE.md` is a minimal adapter that imports this file.

## Purpose

Model Policy is security clearance for AI models: match a business need — task type, processing purpose, data sensitivity — against an organization's security policy and get an explainable, rule-by-rule verdict for every model (`eligible`, `ineligible`, `indeterminate`). Never a silent default; deny by default; deterministic and replayable.

## Scope / Non-scope

- **Reserved home.** This repository is the reserved home of Model Policy. The product is being rebuilt in the canonical base repository [`libre-ai/libre-ai`](https://github.com/libre-ai/libre-ai) (multi-repo topology, [ADR-0008](https://github.com/libre-ai/libre-ai/blob/main/docs/adr/0008-multi-repo-target-topology-and-brand.md)); it reopens as the real product repository when the owner activates it (wave 4). Per the README, it stays private until a secrets audit clears it for public reopening.
- The legacy implementation carried here (Rust workspace `crates/{domain,policy,dataset,sync,cli,api}` + `apps/web`, Playwright e2e harness) is **frozen for reference**.
- Non-scope: new product development in this repository until activation.

## Commands

Verified against `Cargo.toml`, `package.json`, `scripts/`, and `.github/workflows/`:

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --all-targets`
- `cargo llvm-cov --workspace --ignore-filename-regex 'main\.rs' --fail-under-lines 80` — coverage gate (CI)
- e2e: `dx build --platform web --release`, then `npm ci && npx playwright install --with-deps && npx playwright test` (or `npm run e2e`)
- `./scripts/build-demo.sh` — demo build

## CI gates

- `ci` (`.github/workflows/ci.yml`) — jobs: `rust`, `deny`, `coverage`.
- `e2e` (`.github/workflows/e2e.yml`) — job: `web`.
- `hygiene` (`.github/workflows/hygiene.yml`).
- `Context hygiene` (`.github/workflows/context-hygiene.yml`).

## Links

- [README](README.md) · [Français](README.fr.md)
- [docs/api.md](docs/api.md) — API documentation
- [docs/deploy-demo.md](docs/deploy-demo.md) — demo deployment
