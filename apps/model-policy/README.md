# @libre-ai/model-policy

Model Policy lets an organisation author, approve and apply deterministic policies
that decide whether a model snapshot satisfies a need, with explainable verdicts.

Work package: `WP-G3-M01`.

## Increment 4 — cockpit (accessible SSR read view)

`src/server/handler.ts` + `src/ui/model-policy-cockpit.tsx` serve the read-only
policies cockpit, server-rendered and usable **without JavaScript**, from a contract
fixture (`src/ui/fixture.ts`). The view **authors nothing and evaluates nothing** —
the deterministic rule evaluation is the deferred Rust/WASM boundary; the cockpit
lists the approved policy definitions and their metadata.

- `createModelPolicyHandler` routes `/` to the SSR document and `/api/health` to a
  JSON status; an unknown route is `404`.
- `ModelPolicyCockpit` renders an ordered, accessible table (a `<caption>`, `scope`
  column/row headers, a skip link, a `main` landmark): each policy's id, version,
  rule count, proposer and approval date — conveyed **as text, never colour**.

Verified: the static render is a well-formed `<!doctype html>` document in French,
the table exposes its caption and header scopes, every fixture policy is listed, and
no inline `style=` carries meaning; the handler serves the cockpit, health, and a 404. Authoring and approval journeys — and the Rust/WASM evaluator — arrive in later
increments.

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
snapshot and need — at authoring time. Deterministic evaluation across them is
wired in Increment 5.

## Increment 5 — WASM evaluation adapter

`src/evaluation/` binds the capability-free `policy-core` WASM component
(`crates/policy-core`, WIT world `policy-core`, #214) as model-policy's runtime
evaluator, matching the spec's runtime boundary: the authorizing **Bun** host
passes canonical policy/snapshot/need bytes plus the explicit evaluation time to a
component that imports **no** host capability (no clock, randomness, network,
filesystem, KV, environment or logging).

- `policy-core-evaluator.ts` — `createPolicyCoreEvaluator()` instantiates the
  component once with an **empty import object**, refusing any core module that
  requests an import, and returns a bytes-in/bytes-out `evaluate(policy, snapshot,
need, evaluatedAt)` matching the WIT exactly. Deterministic: identical input
  bytes yield byte-exact `policy-evaluation.v2` JCS bytes.
- `error-mapping.ts` — lifts the six `policy-core` contract-error codes into a
  typed union, failing **closed** to `engine-unknown` on any unlisted code.

The component is a **server-side** boundary — model-policy is SSR, usable without
JavaScript, with no client bundle; this is not a browser-hosted evaluator like the
local-only Front-C apps.

**Build & verify (server-side WASM):**

```bash
# Build the capability-free component + jco glue → target/policy-core-wasm/ (gitignored).
# Run under node: jco's transpile worker is unsupported by Bun's process bindings.
node tools/quality/build-policy-core-wasm.ts

# Prove the built artifact imports nothing and exports exactly the policy-core API.
cargo run --locked -p policy-core --example check_wasm_imports -- \
  target/wasm32-unknown-unknown/release/policy_core.wasm

# Replay every policy-core-v2 golden vector through the live component (20/20 byte-exact).
bun tools/quality/policy-core-wasm-conformance.ts
```

CI (`rust-quality`) builds the component, runs `check_wasm_imports`, and verifies a
reproducible byte-identical rebuild. The live jco conformance is the pre-merge
qualification check, mirroring notebook-core's out-of-CI qualification harness.

### Deliberately deferred

- The **command surface** around evaluation — `EvaluateEligibility` with Biscuit
  authorization, tenant RLS and persistence; surfacing the evaluation-time codes
  (`snapshot_stale`, `fact_absent`, `engine_version_unknown`,
  `origin_jurisdiction_conflated`) and export
  (`dataset_redistribution_forbidden`) in a UI; content-digest verification, the
  source adapter and the trace/diff UI. The pure evaluator itself is wired above.
- **Source-URI destination safety** (rejecting private/loopback/metadata hosts,
  DNS-rebinding): this validator checks the contract's `https` **shape** only.
  Destination policy is a fetch-time concern owned by the deferred source adapter
  (cf. radar's destination policy), so the validator stays faithful to the locked
  contract rather than stricter than it. A `valid` result is not fetch-authorized.

## License

EUPL-1.2.
