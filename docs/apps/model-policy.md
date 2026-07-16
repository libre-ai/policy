# Model Policy

- **Path:** `apps/model-policy`
- **Purpose:** explainable eligibility of AI models against organizational policy.
- **Runtime:** Bun/React BFF plus Rust/WASM policy core.
- **Owns:** policy editing, needs, snapshots, explanations and UI.
- **Rust owns:** deterministic deny-by-default policy evaluation and sourced snapshot validation.
- **Critical gates:** unknown means ineligible/indeterminate, origin vs jurisdiction separation, provenance, local evaluation and no redistribution of restricted benchmark data.
