# Model Policy product delivery phases

## Status

**Document status: draft planning record.** These records define a proposed product trajectory and acceptance criteria; they contain no current phase, execution status, or passed-gate claim. They grant no implementation, policy, provider-network, credential, production, deployment, commercial, or compliance authority.

[`GOALS.md`](../../../GOALS.md) is the sole program/phase authority and [`STATUS.md`](../../../STATUS.md) is the sole current execution authority. In particular, Model Policy remains behind owner selection, wave 4b activation, and an accepted bounded work package. An agent cannot satisfy those controls by editing this plan.

The machine-readable planning record is [`phases.v1.json`](phases.v1.json), validated by [`phases.v1.schema.json`](phases.v1.schema.json). Gate evidence may be attached only through content-addressed records conforming to [`evidence-record.v1.schema.json`](evidence-record.v1.schema.json); qualified roles additionally bind attestations conforming to [`review-attestation.v1.schema.json`](review-attestation.v1.schema.json). Evidence coverage is not execution status.

## Product progression

<!-- model-policy-plan:start -->
| Phase | Planned product outcome | Depends on | Evidence records |
| --- | --- | --- | ---: |
| [MP-P0](phases/00-foundation.md) | Establish the sourced facts, approved rules, and deterministic evaluator required by every later phase. | — | 0/7 |
| [MP-P1](phases/01-deterministic-qualification.md) | Let a business user qualify a use case and inspect deterministic policy verdicts for deployment configurations without any LLM. | MP-P0 | 0/8 |
| [MP-P2](phases/02-llm-assistance.md) | Pre-fill and challenge the deterministic tunnel while keeping every AI suggestion non-authoritative. | MP-P1 | 0/7 |
| [MP-P3](phases/03-organization-governance.md) | Translate an organization's doctrine into approved, versioned policies and bounded exceptions. | MP-P1 | 0/7 |
| [MP-P4](phases/04-continuous-monitoring.md) | Re-evaluate use cases when model, provider, policy, evidence, or engine facts change. | MP-P3 | 0/7 |
| [MP-P5](phases/05-access-gateway.md) | Enforce approved model routes through revocable access profiles without exposing provider secrets. | MP-P4 | 0/8 |
| [MP-P6](phases/06-activity-cockpit.md) | Pilot evidence health, policy state, access, operations, cost, usage, and incidents without model-quality ranking or compliance scoring. | MP-P4, MP-P5 | 0/8 |
| [MP-P7](phases/07-managed-service.md) | Operate model-policy control under an organization's approved doctrine and explicit service commitments. | MP-P6 | 0/8 |
<!-- model-policy-plan:end -->

MP-P2 is an optional extension after MP-P1, not a dependency of governance or managed operation. MP-P3 through MP-P7 form the deterministic service path. The no-LLM tunnel remains available through every later phase.

## Stable boundaries

One integrated user experience composes distinct authorities:

```text
Deterministic tunnel ── optional assistant suggestions
         │
         ▼
Confirmed bounded need + approved organization policy
         │
         ▼
Pure eligibility evaluator ◄── sourced deployment-configuration snapshot
         │
         ├── eligible / ineligible / indeterminate + rule evidence
         ▼ authorized approval command
Versioned access profile ── per-consumer credentials ── gateway
         │
         ▼
Privacy-minimized activity cockpit and evidence exports
```

- Eligibility is not procurement, recommendation, approval, or authorization.
- The plan adds no benchmark leaderboard, model-quality oracle, or procurement ranker.
- An access profile is shareable; a secret credential is individual.
- Provider credentials never leave the gateway boundary.
- Model origin, provider entity, hosting jurisdiction, processing location, and certification evidence are separate facts.
- Historical evidence is immutable; new facts produce new evaluations.
- Restriction may be automatic, but access broadening requires approval.

## Planning vocabulary

- **Tunnel:** versioned deterministic question graph; never a conversational authority.
- **Passport:** confirmed, digest-bound bounded need for one use case.
- **Doctrine:** customer-owned source material from which humans approve machine policy.
- **Deployment configuration:** complete model, provider, contractual, location, retention, and processing route—not a model name.
- **Access profile:** approved logical permission set; never a secret.
- **Qualified:** independently evidenced to the declared gate level; not a legal-compliance label.

## Supporting authorities

- [Product specification](../model-policy.md) — current purpose, actors, protocol, contracts, and non-goals.
- [Metric catalogue](METRICS.md) — stable definitions/formulas, kinds, introduction phases, privacy classes, and gate use.
- [Evidence policy](EVIDENCE.md) — evidence levels, review roles, storage, invalidation, and customer export.
- [Policy Core v2 semantics](../../../contracts/wit/policy-core-v2/SEMANTICS.md) — deterministic eligibility boundary.
- [Role-separated review protocol](../../reviews/AGENT-REVIEW-PROTOCOL.md) — immutable review requirements.
- [Initial independent review remediation](../../reviews/model-policy-product-phases/REMEDIATION.md) — rejected candidate findings and integrated corrections.

Architecture decisions remain in [`docs/adr/`](../../adr/), contracts remain in [`contracts/`](../../../contracts/), and non-normative evidence never silently becomes doctrine.

## Planning and evidence workflow

1. Define or amend a phase gate in its phase record.
2. Obtain owner selection, wave activation, accepted work package, and any required contract/ADR/specification amendment through their existing authorities.
3. Implement only inside the accepted write paths.
4. Collect candidate-commit artifacts, the canonical gate-section digest, exact tools, commands, inputs, harnesses, and observed results.
5. Run every phase-required independent review and content-address its role/candidate/gate attestation plus human report.
6. Produce the final evidence record for the exact phase, gate, candidate, assertion, achieved level, and attestations; an `in_service` record also binds distinct structured JSON for operated-environment identity, favorable deployment authorization, bounded observations, passing smoke/rollback, and resolved incidents.
7. Let `GOALS.md`/`STATUS.md` record any accepted execution-state change; this plan never self-promotes.
8. Regenerate both README projections only after validation succeeds:

```console
bun apps/model-policy/tools/check-product-phases.ts --write
bun run --cwd apps/model-policy test:product-phases:coverage
bun tools/quality/check-secret-scan.ts
```

The app phase-plan check and its repository-discovered Bun integration tests refuse mutable-worktree substitution for indexed roadmap/schema/phase authorities, schema, dependency, evidence/attestation binding and digest, canonical gate-section drift, source-commit blob, required-review, operational kind/outcome/identity/window/time reuse or mismatch, sensitive evidence content, bidirectional gate-definition, or README drift failures. The checker stages both projections before replacement and rolls back an applied replacement if a later one fails. A repository-discovered app test executes the app-local coverage thresholds, so the existing root `bun test` CI path blocks a coverage regression without changing shared integrator configuration.
