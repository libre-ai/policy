> [!WARNING]
> **Frozen on 2026-07-16 — reserved as the future home of Model Policy ([monorepo ADR-0008](https://github.com/libre-ai/libre-ai/blob/main/docs/adr/0008-multi-repo-target-topology-and-brand.md)).**
> Model Policy is being rebuilt from locked contracts in the canonical base repository [`libre-ai/libre-ai`](https://github.com/libre-ai/libre-ai) (target: `apps/model-policy`). This repository will reopen as the real product repository when the owner activates it. Everything below describes the pre-freeze state and no longer reflects the current architecture or roadmap.

# Policy

Canonical repository: [`libre-ai/policy`](https://github.com/libre-ai/policy). The historical `rumble-ai-clearance-*` crate, binary and document identifiers remain stable technical contracts.

Security clearance for AI models. Match a business need (task type, processing
purpose, data sensitivity) against your organisation's security policy (banned
countries, jurisdictions, licences, hosting requirements, price) and get an
**explainable, rule-by-rule verdict** for every model: eligible, ineligible, or
indeterminate — never a silent default.

The canonical brief it answers: _"no US, no China, but self-hosted is fine"_ —
expressed as a constraint on **where inference data flows** (jurisdiction,
CLOUD Act aware), independent of **who created the model** (origin). Both
dimensions are first-class and separately constrainable.

## Principles

- **Policy as code**: the security team writes a versioned YAML policy on top
  of a sourced default rulebook; every override and deactivation is named and
  traced. See `schemas/policy.schema.json`.
- **Deny by default**: unknown model, missing data on a required dimension, or
  an undocumented hosting path is never eligible.
- **Filter, then rank**: eligibility (security's domain) is strictly separated
  from benchmark/price ranking (business's domain). A non-compliant model
  never appears in a list "with a warning" — it is out, with its verdict
  consultable.
- **Local-first**: one pure Rust engine, compiled to native (CLI, CI gates,
  self-hosted API) and WASM (browser — your policy never leaves your machine).
- **Reproducible decisions**: every verdict cites its snapshot version;
  snapshots carry per-field-group provenance and dated sources.

## Quickstart

```sh
# 1. Build the org-local snapshot (get a free API key at artificialanalysis.ai)
AA_API_KEY=... cargo run -p rumble-ai-clearance-cli --bin clearance -- sync \
  --governance content/governance/providers.yaml \
  --out data/snapshot.json

# 2. What can I use for summarising documents that contain personal data?
cargo run -p rumble-ai-clearance-cli --bin clearance -- evaluate \
  --rulebook content/rulebook/rulebook.yaml \
  --policy examples/policy-no-us-cn-selfhost-ok.yaml \
  --need examples/need-pii-summary.yaml \
  --snapshot data/snapshot.json

# 3. Gate a model in CI (exit 0 eligible / 1 refused / 2 error)
cargo run -p rumble-ai-clearance-cli --bin clearance -- check meta/llama-4 \
  --rulebook content/rulebook/rulebook.yaml \
  --policy examples/policy-no-us-cn-selfhost-ok.yaml \
  --need examples/need-code-public.yaml \
  --snapshot data/snapshot.json

# 4. Why is a model refused?
cargo run -p rumble-ai-clearance-cli --bin clearance -- explain openai/gpt-6 \
  --rulebook content/rulebook/rulebook.yaml \
  --policy examples/policy-no-us-cn-selfhost-ok.yaml \
  --need examples/need-pii-summary.yaml \
  --snapshot data/snapshot.json
```

Air-gapped / offline: `clearance sync --aa-file <recorded.json> --hf-file
<recorded.json> --generated-at <rfc3339>` builds a reproducible snapshot
without touching the network.

## Components

```
crates/domain      pure eligibility engine (native + wasm), property-tested
crates/policy      rulebook ⊕ org merge, traced deactivations, fail-closed
crates/dataset     snapshot format: manifest, provenance, atomic writes
crates/sync        AA / HF / curated connectors (fixtures-tested, no live CI)
crates/cli         clearance: sync | validate | evaluate | explain | check
crates/api         clearance-api: read-only HTTP API (see docs/api.md)
apps/web           Dioxus UI, dual mode, Libre IA Design System 2.0 via Portal
content/           default rulebook (sourced) + curated provider governance
schemas/           JSON Schema contracts: policy, governance
examples/          example org policy + need profiles
```

## Policy model

Three documents, one effective policy:

1. **Default rulebook** (`content/rulebook/rulebook.yaml`, shipped, sourced):
   C2 → EU-27 jurisdiction or self-host; C3 → self-host only; personal data →
   EU-27 or self-host (RGPD ch. V); health data → self-host only (RGPD
   art. 9); per-task ranking dimensions per the Artificial Analysis indices.
2. **Org policy** (yours, versioned in your repo): bans by origin /
   jurisdiction / provider / licence openness, thresholds per sensitivity,
   overrides — and deactivations of rulebook rules **with a mandatory traced
   reason**.
3. **Need profile** (task + purpose + sensitivity): what the business user
   wants to do.

## Data sources and licence boundaries

| Dataset                                                                  | Source                                                                           | Distribution                                                        |
| ------------------------------------------------------------------------ | -------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| Catalogue (identity, licences, weights, modalities)                      | Hugging Face public API                                                          | Org snapshot                                                        |
| Governance (provider HQ country, openness class, self-hostability)       | Curated + sourced, `content/governance/`                                         | **Public, in this repo** — corrections welcome by PR                |
| Benchmarks (intelligence/coding/agentic/math/multilingual, price, speed) | [Artificial Analysis](https://artificialanalysis.ai/) Data API, **your own key** | **Never distributed** — org-local `data/` (gitignored, CI-enforced) |

Benchmark data © [Artificial Analysis](https://artificialanalysis.ai/) — used
under their free-tier terms (internal use only, attribution required). This
project ships the sync pipeline, not their data. The public demo therefore
shows an illustrative catalogue with empty benchmark columns.

## Development

```sh
cargo test --workspace          # unit, property-based, golden, e2e CLI/API
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check                # licences (AGPL/SSPL banned), advisories

# Web UI — self-hosted fonts and versioned Portal/Libre IA assets
cd apps/web && dx build --platform web --release
npx playwright test             # chromium + firefox + webkit
```

CI gates: fmt, clippy zero-warnings, tests, cargo-deny, blocking coverage,
hygiene (secret smoke, no machine-local paths, no AA data committed), e2e.

Design spec: `docs/superpowers/specs/2026-07-10-rumble-ai-clearance-design.md`.
HTTP API: `docs/api.md`.

## License

MIT
