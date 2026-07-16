# Model Policy

- **Path:** `apps/model-policy`
- **Owner:** Experiences / Model Policy
- **Runtime:** Bun.serve/React BFF plus Rust policy core compiled to WASM
- **Tenant model:** organization

## Purpose and actors

Model Policy lets an organization define sourced eligibility rules, capture a versioned model/provider snapshot and obtain a deterministic deny-by-default verdict with traceable explanations. Policy editors propose; approvers accept immutable policy versions; evaluators compare a declared need and snapshot.

## Journeys

1. **Author/approve policy:** editor creates rules with sources and unknown handling; approver reviews diff and accepts immutable version.
2. **Build snapshot:** evaluator imports sourced model/provider facts, validates provenance/licence/date and freezes snapshot hash.
3. **Evaluate need:** user describes bounded requirements, runs local deterministic evaluation and inspects verdict, failed/unknown rules and evidence.
4. **Export/replay:** user exports need/policy/snapshot/result hashes and replays with same engine version for identical result.

## Non-goals

- benchmark leaderboard, model quality oracle or procurement decision automation ;
- inferring jurisdiction from company origin ;
- scraping or redistributing restricted benchmark/licence data ;
- provider network calls during deterministic evaluation ;
- treating missing facts as compliant.

## Domain protocol

**Commands:** `CreatePolicyDraft`, `AddPolicyRule`, `SubmitPolicyForApproval`, `ApprovePolicyVersion`, `ImportModelSnapshot`, `ValidateSnapshot`, `CreateNeed`, `EvaluateEligibility`, `ExportEvaluation`, `RevokeSnapshot`.

**Queries:** `GetPolicyVersion`, `DiffPolicyVersions`, `GetSnapshot`, `ListSnapshotEvidence`, `GetEvaluation`, `ExplainRuleResult`, `ReplayEvaluation`.

**Events:** `PolicyDraftCreated`, `PolicyVersionApproved`, `SnapshotImported`, `SnapshotValidated`, `SnapshotRejected`, `EligibilityEvaluated`, `EvaluationExported`, `SnapshotRevoked`.

Verdict is `eligible | ineligible | indeterminate`; only all mandatory rules satisfied with complete required facts yields `eligible`. Revocation prevents new evaluation but does not rewrite past evidence.

## Refusal matrix

| Code | Refusal |
| --- | --- |
| `policy.version_unapproved` | evaluation requests draft/unapproved policy |
| `policy.snapshot_unsourced` | required fact lacks source/provenance |
| `policy.snapshot_stale` | source age exceeds rule bound |
| `policy.origin_jurisdiction_conflated` | snapshot maps origin directly to jurisdiction |
| `policy.fact_unknown` | required fact absent; result becomes indeterminate/ineligible per rule |
| `policy.rule_unbounded` | rule uses unsupported/non-deterministic operation |
| `policy.dataset_redistribution_forbidden` | export would include restricted source payload |
| `policy.engine_version_unknown` | no qualified evaluator for contract/engine version |
| `policy.tenant_mismatch` | policy/snapshot/need tenant differs |

The UI may explain indeterminate but cannot override it to eligible.

## Data

PostgreSQL owns organization policies, immutable accepted versions, source references, snapshot facts/provenance, needs and evaluation manifests. Restricted source payloads are not stored unless licence explicitly permits and owner approves; only citations/digests and derived allowed facts persist. Accepted policy/snapshot records follow ADR-0002 section 3 retention and remain immutable while referenced. Migration source is selected public model-policy/policy contracts and accepted source datasets, not historical private tables.

## Authentication and authorization

All Model Policy v1 reads, editing and evaluation require an opaque organization session and tenant. A future public reference policy is a reviewed Website projection, not a public Model Policy API exception. Biscuit resources are `policy/<id>/<version>`, `snapshot/<id>` and `evaluation/<id>`; an editor cannot approve their own version in v1. Rust policy core receives no token; Bun authorizes then passes canonical policy/need/snapshot bytes. RLS repeats tenant isolation.

## Runtime boundaries

TypeScript owns authoring, approval, persistence, source adapter and explanations UI. Rust owns schema-validated pure evaluation, stable rule IDs, deterministic trace and canonical result hashing through WIT/WASM. No HTTP, clock, randomness or DB in the Rust world; evaluation time/source freshness is an explicit input fact.

## Accessibility and degraded mode

Policy diff, verdict and rule trace have structured tables/lists and do not rely on color. Keyboard/screen-reader users can inspect each failed/unknown rule and source. Evaluation runs locally after required artifacts load. Source network outage blocks new snapshot refresh but permits replay of accepted snapshots with prominent age; revoked snapshots fail closed.

## Contracts

- Policy Definition v1 — `contracts/schemas/policy-definition.v1.schema.json` ;
- Model Snapshot v1 — `contracts/schemas/model-snapshot.v1.schema.json` ;
- Policy Evaluation v1 — `contracts/schemas/policy-evaluation.v1.schema.json` ;
- Model Policy API — `contracts/openapi/model-policy.v1.yaml` ;
- pure evaluator — `contracts/wit/policy-core-v1/world.wit`.

## Evidence

Golden vectors cover every operator, unknown path, source age and origin/jurisdiction distinction in Rust and TypeScript. Property tests prove order independence and deny-by-default. Contract fixtures include restricted payload and cross-tenant negatives. E2E covers author/approve/import/evaluate/export/replay. Supply-chain/licence gate checks every bundled dataset.

## Work packages

1. policy/snapshot/evaluation schemas and golden vectors — Canonical Core ;
2. deterministic Rust evaluator and WIT — Specialized Rust ;
3. tenant authoring/approval/source persistence — Experiences ;
4. React trace/diff/local evaluation — Experiences + Web Platform ;
5. provenance/licence/determinism qualification — Proof + Infrastructure and Release.

Rust and UI may proceed in parallel only against accepted golden vectors.

## Release and rollback

Release requires cross-runtime golden equality, unknown/indeterminate proof, independent policy approval, source provenance/licence checks and RLS. Policy/snapshot/evaluation contracts remain readable after rollback. A faulty engine is revoked for new runs and previous engine/artifact restored; historical result hashes remain unchanged and visibly identify engine version.
