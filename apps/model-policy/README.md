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

### Deliberately deferred

- The deterministic **rule evaluator** (Rust/WASM boundary) and the
  evaluation-time codes (`snapshot_unsourced`, `snapshot_stale`, `fact_absent`,
  `engine_version_unknown`, `origin_jurisdiction_conflated`), the cross-input
  `tenant_mismatch`, export (`dataset_redistribution_forbidden`), persistence, the
  source adapter and the UI.
- **Source-URI destination safety** (rejecting private/loopback/metadata hosts,
  DNS-rebinding): this validator checks the contract's `https` **shape** only.
  Destination policy is a fetch-time concern owned by the deferred source adapter
  (cf. radar's destination policy), so the validator stays faithful to the locked
  contract rather than stricter than it. A `valid` result is not fetch-authorized.

## License

EUPL-1.2.
