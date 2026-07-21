# @libre-ai/model-policy

Model Policy lets an organisation author, approve and apply deterministic policies
that decide whether a model snapshot satisfies a need, with explainable verdicts.

Work package: `WP-G3-M01`.

## Increment 1 — approved policy-definition validator

`src/domain/policy-definition.ts` is the pure, offline validator for an approved
`policy-definition.v2`. TypeScript owns authoring and human approval; the
deterministic rule **evaluator** is the deferred Rust/WASM boundary — **this
module implements no evaluation**. It reuses the locked `common.v1` /
`policy-definition.v2` definitions verbatim.

`validatePolicyDefinition(input)` returns a three-state result:

| Status      | Meaning                                                                                             |
| ----------- | --------------------------------------------------------------------------------------------------- |
| `valid`     | a typed, contract-conformant approved policy (deep-frozen)                                          |
| `malformed` | not a well-formed policy (identity/structure fails the schema) — a boundary concern, no matrix code |
| `refused`   | well-formed but violates a domain invariant, with the exact `policy.*` code                         |

Semantic refusals:

- `policy.version_unapproved` — the status is not `approved` (a draft/unapproved
  policy must not be applied).
- `policy.rule_unbounded` — a rule uses an unsupported operator, a value that does
  not match its operator (`equals`/`not-equals` → scalar, `in`/`not-in` →
  homogeneous unique set, `at-least`/`at-most` → number), or a source-age bound on
  a non-`model.` fact.

## Increment 2 — model-snapshot validator

`src/domain/model-snapshot.ts` validates the other authoring-time input to
evaluation: a `model-snapshot.v2`, the sourced, content-addressed set of facts
about a model. `validateModelSnapshot(input)` returns the same three-state
result:

| Status      | Meaning                                                                                    |
| ----------- | ------------------------------------------------------------------------------------------ |
| `valid`     | a typed, contract-conformant snapshot (deep-frozen)                                        |
| `malformed` | identity/structure fails the schema — boundary concern, no matrix code                     |
| `refused`   | a well-formed snapshot with a fact that lacks a valid source → `policy.snapshot_unsourced` |

Each fact is `{name (model.*), value (scalar), source}`; a fact whose name/value
is structurally bad is `malformed`, while a fact missing or with an invalid
`source` is `snapshot_unsourced`. Fact names are the stable identity (duplicates
refused). Content-digest verification is deferred.

## Increment 3 — policy-need validator

`src/domain/policy-need.ts` validates the **third** evaluation input, a
`policy-need.v2` — the set of `need.*` facts a policy is evaluated against.
`validatePolicyNeed(input)` is **two-state** (`valid` / `malformed`): unlike a
snapshot, need facts carry no source, and the refusal matrix defines no
authoring-time semantic refusal for a need's structure, so any identity or
fact-structure failure is `malformed` and a conformant need is `valid`. Fact
names are the stable identity (duplicates refused).

With this, **model-policy validates all three evaluation inputs** — policy,
snapshot and need — at authoring time; deterministic evaluation across them
remains the deferred Rust/WASM boundary.

### Deliberately deferred

- The deterministic **rule evaluator** (Rust/WASM boundary) and the
  evaluation-time codes (`snapshot_stale`, `fact_absent`, `engine_version_unknown`,
  `origin_jurisdiction_conflated`), the cross-input `tenant_mismatch`, export
  (`dataset_redistribution_forbidden`), content-digest verification, persistence,
  the source adapter and the UI.
- **Source-URI destination safety** (rejecting private/loopback/metadata hosts,
  DNS-rebinding): this validator checks the contract's `https` **shape** only.
  Destination policy is a fetch-time concern owned by the deferred source adapter
  (cf. radar's destination policy), so the validator stays faithful to the locked
  contract rather than stricter than it. A `valid` result is not fetch-authorized.

## License

EUPL-1.2.
