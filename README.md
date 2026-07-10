# rumble-ai-clearance

Security clearance for AI models. Match a business need (task type, processing
purpose, data sensitivity) against an organisation's security policy (banned
countries, jurisdictions, licences, hosting requirements, price) and get an
**explainable, rule-by-rule verdict** for every model: eligible, ineligible, or
indeterminate — never a silent default.

- **Policy as code**: the security team writes a versioned YAML policy on top of
  a sourced default rulebook; every override and deactivation is named and
  traced.
- **Origin ≠ hosting**: "no US, no China, but self-hosted is fine" expresses
  naturally — model origin and inference data path are independent dimensions,
  jurisdiction included (CLOUD Act awareness).
- **Deny by default**: unknown model or missing data on a required dimension is
  never eligible.
- **Filter, then rank**: eligibility (security's domain) is strictly separated
  from benchmark/price ranking (business's domain).
- **Local-first**: one pure Rust engine, compiled to native (CLI, CI gates,
  self-hosted API) and WASM (browser — your policy never leaves your machine).

## Data sources

| Dataset                                                                          | Source                                                                           | Distribution                                                                         |
| -------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| Catalogue (identity, licences, weights, modalities)                              | Hugging Face public API                                                          | Versioned in this repo's snapshots                                                   |
| Governance (provider country, openness class, self-hostability)                  | Curated, sourced, in `content/governance/`                                       | Versioned in this repo                                                               |
| Benchmarks (intelligence/coding/agentic/math/multilingual indices, price, speed) | [Artificial Analysis](https://artificialanalysis.ai/) Data API, **your own key** | **Never distributed** — each org builds its own local snapshot (`data/`, gitignored) |

Benchmark data © [Artificial Analysis](https://artificialanalysis.ai/) — used
under their free-tier terms (internal use only, attribution required). This
project ships the sync pipeline, not their data.

## Status

Under construction — see `docs/superpowers/specs/` for the validated design.

## License

MIT
