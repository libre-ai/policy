**English** · [Français](README.fr.md)

> [!NOTE]
> **Reserved · future home of Model Policy** — rebuilt in the canonical base repository [`libre-ai/libre-ai`](https://github.com/libre-ai/libre-ai) ([multi-repo topology, ADR-0008](https://github.com/libre-ai/libre-ai/blob/main/docs/adr/0008-multi-repo-target-topology-and-brand.md)).
> This repository will reopen as the real product repository when the owner activates it, consuming the base as a versioned dependency. The foundations described below are **being built now** — with links to the code that already exists.

# Model Policy

**Security clearance for AI models.** Match a business need — task type, processing purpose, data sensitivity — against your organization's security policy — banned countries, jurisdictions, licences, hosting requirements, price — and get an **explainable, rule-by-rule verdict** for every model: `eligible`, `ineligible`, or `indeterminate`. Never a silent default.

The canonical brief it answers: _"no US, no China, but self-hosted is fine"_ — expressed as a constraint on **where inference data flows** (jurisdiction, CLOUD Act aware), independent of **who created the model** (origin). Both dimensions are first-class and separately constrainable.

## Why it's different

- **Explainable, not a score.** Every verdict is traceable rule by rule — you see _which_ rule failed and _why_, on which sourced fact. A non-compliant model is out, with its reasoning consultable — never listed "with a warning".
- **Deny by default.** An unknown model, a missing fact on a required dimension, or an undocumented hosting path is never `eligible`.
- **Deterministic and replayable.** The same policy, snapshot and need always produce byte-identical evidence. Verdicts are reproducible and auditable, not a model's opinion.
- **Filter, then rank.** Eligibility (security's domain) is strictly separated from benchmark/price ranking (business's domain).
- **Sourced facts only.** Policies are built on a sourced rulebook; every override is named and traced. Model facts carry provenance.

## Status — spec-published, foundations under construction

Model Policy is being rebuilt from locked contracts. It is **not released yet**; the deterministic evaluation core comes first, and a good part of it already exists and is proven in the base repository:

| Foundation                                                  | State                | Evidence                                                                                                                                       |
| ----------------------------------------------------------- | -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| **`policy-core`** — deterministic Rust evaluation engine    | ✅ built, byte-exact | 20/20 golden-vector conformance against normative `SEMANTICS.md` ([#212](https://github.com/libre-ai/libre-ai/pull/212))                       |
| **`policy-core` → WASM** — capability-free component        | ✅ built             | No host imports — no clock, network, filesystem, randomness or identity ([#214](https://github.com/libre-ai/libre-ai/pull/214))                |
| **Server-side evaluation** — the app consumes the component | ✅ wired             | Bun host instantiates the WASM and evaluates; live 20/20 byte-exact ([#215](https://github.com/libre-ai/libre-ai/pull/215))                    |
| **`policy-core-ref`** — TypeScript reference evaluator      | ✅ published         | Byte-identical semantics, 144-vector conformance ([#207](https://github.com/libre-ai/libre-ai/pull/207))                                       |
| Authoring validators — policy / snapshot / need             | ✅ built             | Fail-closed, contract-conformant ([#169](https://github.com/libre-ai/libre-ai/pull/169)–[#181](https://github.com/libre-ai/libre-ai/pull/181)) |
| Command surface — authorize, persist, export, trace UI      | ⏳ next              | Biscuit authorization, tenant isolation, replay evidence                                                                                       |

This repository is `private` until a secrets audit clears it for public reopening (wave 4). **Benchmark target:** model-registry / model-card governance tooling (e.g. Hugging Face Hub) — reached through explainable, deny-by-default clearance rather than discovery.

## How it works

1. **Author** — editors write a versioned policy of sourced eligibility rules over models' facts; approvers accept **immutable** policy versions (a proposer cannot approve their own).
2. **Snapshot** — import sourced model/provider facts, validate provenance and licence, and freeze a content-addressed snapshot.
3. **Evaluate** — declare a bounded need, run **local, deterministic** evaluation, and inspect the verdict with its failed and unknown rules and their evidence. Revocation blocks new evaluation but never rewrites past evidence.

## Architecture — built from interoperable bricks

Model Policy is a product assembled from independently versioned bricks; each is usable and testable on its own, and the product is their composition (the multi-repo target of [ADR-0008](https://github.com/libre-ai/libre-ai/blob/main/docs/adr/0008-multi-repo-target-topology-and-brand.md)).

| Brick                                        | Role                                          | Interface it exposes / consumes                                                                                                 |
| -------------------------------------------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| **`policy-core`** (Rust → WASM component)    | The deterministic evaluation engine           | WIT world `policy-core`: `evaluate(policy, snapshot, need, evaluated-at) → evaluation`, capability-free                         |
| **`@libre-ai/policy-core-ref`** (TypeScript) | Reference evaluator, byte-identical semantics | Same evaluation contract, for cross-checking and JS-side use                                                                    |
| **`@libre-ai/web-platform`**                 | SSR / Bun BFF foundation                      | Request handler, accessible server-rendered document                                                                            |
| **Contracts**                                | Locked interoperability surface               | `policy-definition.v2`, `model-snapshot.v2`, `policy-need.v2`, `policy-evaluation.v2` schemas + golden vectors + `SEMANTICS.md` |

The authorizing host passes canonical policy/snapshot/need bytes to the engine; the engine holds no token and reaches no capability. Any consumer that speaks the same contracts can drive the same evaluation.

## Where the work happens

All active development is in the base repository, under:

- `apps/model-policy` — the product host (SSR cockpit, server-side evaluation)
- `crates/policy-core` — the Rust engine and its WASM component
- `packages/policy-core-ref` — the TypeScript reference evaluator
- `contracts/` — the locked schemas, WIT world and golden vectors
- [`docs/apps/model-policy.md`](https://github.com/libre-ai/libre-ai/blob/main/docs/apps/model-policy.md) — the full product brief

To follow progress or contribute, open issues and pull requests in [`libre-ai/libre-ai`](https://github.com/libre-ai/libre-ai). This repository stays reserved until activation.

## License

EUPL-1.2.
