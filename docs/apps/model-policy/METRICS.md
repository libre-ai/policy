# Model Policy metric catalogue

## Authority and rules

This catalogue defines product metric meaning; it does not contain organization-instance values. A metric is usable for a gate or accountable decision support only when its producer, source, window, freshness, unit, missing-value behavior, privacy class, and version are recorded with the observation.

- **Gate** metrics block promotion at their stated threshold.
- **Operational** metrics describe service health but do not alter eligibility.
- **Business** metrics inform accountable humans but never rank models, compensate for failed rules, or grant access.
- Missing is always unknown, never zero.
- A definition or formula change creates a new metric version.
- Labels use opaque bounded IDs; prompts, responses, documents, credentials, and direct person identifiers are forbidden.

## Determinism, safety, and provenance

| ID | Kind | Definition and formula | Gate | Introduced | Privacy |
| --- | --- | --- | --- | --- | --- |
| `MP-MET-DET-001` | Gate | Exact replay equality: byte-identical successful outputs / repeated evaluations with identical policy, need, snapshot, engine, and instant | `100%` | MP-P0 | None |
| `MP-MET-SAF-001` | Gate | Eligible evaluations containing any failed or unknown mandatory rule | `0` | MP-P0 | None |
| `MP-MET-PROV-001` | Gate | Required model facts with accepted source, digest, licence, and retrieval instant / all required model facts | `100%` | MP-P0 | None |
| `MP-MET-FRESH-001` | Gate/operational | Required facts within their policy freshness bound / required facts examined; stale facts remain separately counted | `100%` for `eligible` | MP-P0 | None |
| `MP-MET-PII-001` | Gate | Prompts, responses, documents, credentials, or direct person identifiers found in logs/telemetry/evaluation exports | `0` | MP-P0 | Potential incident only; findings are not echoed |

## Deterministic tunnel and explanation

| ID | Kind | Definition and formula | Gate or use | Introduced | Privacy |
| --- | --- | --- | --- | --- | --- |
| `MP-MET-TUN-001` | Gate | Mandatory question-path coverage: tested reachable answer states / all reachable answer states in the versioned graph | `100%` | MP-P1 | None |
| `MP-MET-TUN-002` | Product | Confirmed passports reaching a deterministic result / started passports, segmented by explicit abandonment and unresolved mandatory facts | Diagnostic; never weakens questions | MP-P1 | Opaque passport/session IDs only |
| `MP-MET-EXPL-001` | Gate | Displayed verdict/rule/source rows matching canonical evaluation and snapshot facts / rows checked | `100%` | MP-P1 | None |

`MP-MET-TUN-002` measures usability, not correctness. It must not be optimized by hiding unknowns, removing mandatory questions, or preselecting permissive answers.

## LLM assistance

| ID | Kind | Definition and formula | Gate or use | Introduced | Privacy |
| --- | --- | --- | --- | --- | --- |
| `MP-MET-AI-001` | Qualification | Per-field precision and recall against the locked labelled corpus; reported by fact and risk direction, never only as one average | Threshold accepted per field before promotion | MP-P2 | Synthetic/approved corpus only |
| `MP-MET-AI-002` | Gate | Assistant omissions or risk-lowering suggestions that can reach evaluation without explicit correction/confirmation | `0` by architecture and tests | MP-P2 | None in production telemetry |
| `MP-MET-AI-003` | Product | Accepted, edited, and rejected suggestions / suggestions shown, by bounded fact ID | Diagnostic, not an accuracy target | MP-P2 | Opaque passport ID; no source text |

Inter-run assistant consistency is recorded during qualification but is not an evaluator invariant. Identical confirmed facts, not identical generative prose, define deterministic product behavior.

## Organization governance

| ID | Kind | Definition and formula | Gate or use | Introduced | Privacy |
| --- | --- | --- | --- | --- | --- |
| `MP-MET-GOV-001` | Gate | Approved policies/exceptions whose proposer differs from the human approver and whose approval binds the exact subject digest | `100%` | MP-P3 | Opaque actor IDs |
| `MP-MET-GOV-002` | Gate/operational | Active exceptions with valid scope, approval, expiry, and compensating-control record / active exceptions | `100%`; expired active exception count `0` | MP-P3 | Opaque actor/use-case IDs |

## Monitoring and re-evaluation

| ID | Kind | Definition and formula | Gate or use | Introduced | Privacy |
| --- | --- | --- | --- | --- | --- |
| `MP-MET-WATCH-001` | Gate/SLA | Material accepted change events that produce all required affected evaluations within the committed window / material events | `100%` inside configured window | MP-P4 | Opaque tenant/passport/configuration IDs |
| `MP-MET-WATCH-002` | Gate | Unsafe transition events lacking exact old/new evaluation IDs, affected route IDs, severity, authority version, or idempotency key when published to authorization consumers | `0` | MP-P4 | Opaque profile/configuration IDs |
| `MP-MET-WATCH-003` | Operational | Open remediation actions by severity and age, including unassigned, overdue, escalated, accepted-risk, and evidence-closed counts | No unassigned critical action; severity-specific due times defined before activation | MP-P4 | Opaque action/owner-role IDs |
| `MP-MET-WATCH-004` | Gate | Accepted source changes absent from the material-change reconciliation ledger or classified by an unversioned taxonomy | `0` | MP-P4 | None |

The propagation and remediation windows are versioned service parameters. Reporting an average cannot hide a breached hard maximum. MP-P4 publishes evidence and transition events; only MP-P5 measures and enforces runtime denial.

## Access and gateway

| ID | Kind | Definition and formula | Gate or use | Introduced | Privacy |
| --- | --- | --- | --- | --- | --- |
| `MP-MET-ACCESS-001` | Gate/operational | Gateway decisions attributable to one active organization/profile/consumer credential/configuration tuple / gateway decisions | `100%` | MP-P5 | Opaque IDs only |
| `MP-MET-ACCESS-002` | Gate/SLA | Time from accepted revocation or unsafe transition to denial at every gateway instance; publish max and percentile | Hard maximum defined before activation | MP-P5 | Opaque route/profile IDs |
| `MP-MET-ACCESS-003` | Gate | Successful requests using revoked, expired, cross-tenant, out-of-profile, or never-approved routes | `0` | MP-P5 | Opaque IDs only |

## Evidence health, operations, and cost

| ID | Kind | Definition and formula | Gate or use | Introduced | Privacy |
| --- | --- | --- | --- | --- | --- |
| `MP-MET-EVID-001` | Operational | Current, stale, unknown, revoked, and conflicting required facts by policy/configuration, with source and freshness version | Evidence-health action queue; never a model-quality or compliance score | MP-P6 | Opaque policy/configuration IDs |
| `MP-MET-OPS-001` | Operational | Latency p50/p95/p99, throughput, error/retry/fallback rates, and availability over an explicit window | SLOs per profile/route class | MP-P6 | Opaque IDs and bounded numeric units |
| `MP-MET-OPS-002` | Gate | Operational series with unbounded labels, content-bearing labels, unknown unit/window, or missing metric version | `0` | MP-P6 | None; scanner does not echo content |
| `MP-MET-COST-001` | Business | Actual and projected total cost by passport/profile/configuration, including input/output, OCR, storage, retries, fallback, and minimum infrastructure; assumptions and price date mandatory | Budget review input, never automated procurement or eligibility compensation | MP-P6 | Opaque IDs and monetary values |

Provider claims and product observations use distinct source kinds. The cockpit reports operational behavior of already approved routes; it does not infer model quality, compare models, or recommend procurement.

## Managed service

| ID | Kind | Definition and formula | Gate or use | Introduced | Privacy |
| --- | --- | --- | --- | --- | --- |
| `MP-MET-SVC-001` | SLA | Service commitments met / commitments due, reported separately for freshness, re-evaluation, revocation, availability, incident notice, deletion, and export | Contract-specific; no blended score | MP-P7 | Opaque organization IDs |
| `MP-MET-SVC-002` | Gate | Critical managed actions lacking attributable customer/operator responsibility, required approval, or evidence | `0` | MP-P7 | Opaque actor and organization IDs |

## Collection and retention

Metric producers MUST:

1. validate bounded dimensions before emission;
2. reject content and secrets rather than redact after persistence;
3. attach metric/version, source kind, window, and observed instant;
4. retain only for the accepted product-data lifecycle;
5. support organization deletion and aggregate evidence without restoring deleted content;
6. distinguish unavailable, stale, and zero;
7. expose cursor-bounded queries and prevent N+1 retrieval;
8. preserve historical metric-version interpretation.

Any future metric that influences eligibility requires a sourced policy/model/need fact and deterministic rule; adding it only to the cockpit cannot change a verdict.
