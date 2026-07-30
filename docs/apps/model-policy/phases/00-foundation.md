# MP-P0 — Foundation of authority

## Outcome

Establish the bounded facts, approved rules, sourced deployment-configuration evidence, and pure evaluator required by every product journey. This phase makes later decisions reproducible; it delivers no model recommendation, credential, provider call, or production authorization.

## User promise

A security or privacy reviewer can inspect the exact vocabulary, policy, model evidence, evaluator semantics, and unknown handling that a future business-use-case evaluation will apply. Every accepted claim has a source and every missing required fact fails closed.

## Actors

- **Policy editor:** proposes sourced organization rules.
- **Policy approver:** a distinct human who accepts an immutable policy version.
- **Registry curator:** imports and validates facts about a model deployment configuration.
- **Engine qualifier:** proves deterministic, bounded behavior across implementations.
- **Security/privacy reviewer:** challenges provenance, data boundaries, and legal overclaims.

## Inputs and outputs

Inputs are public or organization-approved rule sources, provider/model evidence, an explicit evaluation instant, and bounded machine facts. Outputs are approved policy versions, sourced configuration snapshots, a need-fact dictionary, deterministic evaluation contracts, golden vectors, and qualification evidence.

Natural-language use-case descriptions, personal content, credentials, and unrestricted provider payloads are forbidden from the pure evaluator. Facts use bounded machine tokens; source citations contain no query, user information, credential, email, or private network destination.

## Required domain model

The authority model MUST separate:

1. business task and intended purpose;
2. affected people and decision impact;
3. confidentiality and business criticality;
4. personal, special-category, criminal, financial, and secret data;
5. model artifact and immutable version;
6. serving provider and contractual entity;
7. inference, storage, log, backup, and support locations;
8. retention, training, subprocessors, and human access;
9. licence, distribution, and self-hosting rights;
10. declared task acceptance criteria plus sourced latency, capacity, and cost snapshots.

A model name is not a deployment configuration. The same artifact served locally and through a remote provider can receive different verdicts. Model origin never supplies hosting jurisdiction.

## Deterministic boundary

For one approved policy, one sourced snapshot, one declared need, one evaluator version, and one evaluation instant, the evaluator emits byte-identical `eligible`, `ineligible`, or `indeterminate` evidence. It has no clock, network, storage, randomness, identity, authorization, ranking, or purchasing capability.

Unknown facts remain unknown. A policy decides whether an unknown produces `indeterminate` or `ineligible`; it can never produce `eligible`. Eligibility remains advisory and grants no access.

## Deliverables

- locked policy, need, snapshot, evaluation, OpenAPI, and WIT contracts;
- a reviewed dictionary for `need.*` and `model.*` facts used by the product tunnel;
- a deployment-configuration identity and evidence model;
- source-adapter requirements, including destination safety and freshness;
- an organization policy pack with explicit unknown handling;
- Rust/WASM and reference evaluator conformance evidence;
- resource ceilings, malformed-input refusals, and order-independence vectors;
- product-phase authority and evidence conventions.

## Non-goals

- business questionnaire or conversational capture;
- model ranking or procurement advice;
- provider credentials or inference traffic;
- production persistence or infrastructure;
- a generic claim that a model or provider is “compliant”;
- deriving AI Act, GDPR, certification, or jurisdiction conclusions from free text.

## Data, security, and degraded mode

Only sanitized sourced facts and opaque actor/tenant identifiers may enter durable authority records. Restricted source bodies are not persisted unless their licence and owner explicitly permit it. Source unavailability prevents refresh but does not alter historical evidence. Stale required facts become unknown. Revocation blocks new evaluations while preserving past records.

## Metrics

Required metrics are `MP-MET-DET-001`, `MP-MET-SAF-001`, `MP-MET-PROV-001`, `MP-MET-FRESH-001`, and `MP-MET-PII-001` from [`../METRICS.md`](../METRICS.md).

## Exit gates

### MP-P0-G01 — Policy Core v2 authorities are locked

Policy, need, model snapshot, evaluation, HTTP, and WIT authorities have separate Architecture, Security, and Privacy approvals plus promotion evidence. This gate does not authorize product implementation or deployment.

### MP-P0-G02 — The product fact dictionary is accepted

Every tunnel answer maps to a documented `need.*` fact; every configuration assertion maps to a `model.*` fact. Dimensions are orthogonal, bounded, typed, non-personal, and include explicit unknown behavior. Ambiguous C1–C4 labels may exist only as derived presentation.

### MP-P0-G03 — Deployment configurations have stable identity

The registry distinguishes model artifact, version, provider route, contractual entity, processing locations, retention/training behavior, and relevant subprocessors. Composite configurations identify every component and data-flow edge; no model name alone can stand for a route.

### MP-P0-G04 — Provenance and source-adapter policy is qualified

Every required model fact has a licensed source, digest, retrieval instant, freshness policy, and safe adapter path. Destination authorization resists private/loopback/metadata targets and DNS rebinding. Origin and jurisdiction conflation is mechanically refused.

### MP-P0-G05 — A reference organization policy is approved

The policy covers mandatory residency, jurisdiction, retention, training, licence, and evidence requirements with explicit handling for absent and stale facts. Proposal and approval are separated and bound to the exact digest.

### MP-P0-G06 — Deterministic evaluator qualification is complete

Rust/WASM and the reference implementation pass the same golden vectors byte-for-byte, malformed inputs fail closed, resource ceilings hold, and order-independent inputs produce the same canonical result.

### MP-P0-G07 — Product authority and evidence records are accepted

The phase plan, metrics catalogue, evidence levels, authority map, and rollback rules receive business, technical, architecture, security/privacy, and owner review. Every accepted gate is bound through a content-addressed evidence record. Acceptance creates planning authority only; `GOALS.md`/`STATUS.md` activation and bounded work packages remain separate owner controls.

## Dependencies and parallel work

Contract/evaluator qualification and fact-dictionary/registry design can progress in parallel once shared fact naming and authority boundaries are fixed. This plan does not start that work: `GOALS.md` must first record owner selection, `STATUS.md` must activate wave 4b, and an accepted bounded work package must authorize exact paths. Any contract amendment follows the existing lock and role-separated review protocol.

## Release and rollback

This phase has no production release. A faulty candidate fact dictionary, policy, snapshot schema, or evaluator version is superseded or revoked; previously accepted bytes and evidence remain available for replay. No historical result is rewritten.
