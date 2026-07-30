# MP-P6 — Activity cockpit

## Outcome

Organization operators pilot their AI use-case portfolio through sourced, privacy-minimized observations for evidence health, policy state, access, performance, availability, cost, usage, remediation, and incidents. The cockpit is observational: it does not rank model quality, recommend procurement, score compliance, or mutate authority.

## User promise

A business owner can see whether an approved route remains active, operationally healthy, and inside its declared budget. Security and privacy reviewers can see evidence freshness, policy exceptions, blocked routes, overdue actions, and upcoming renewals. Operators can connect activity to one passport/profile/consumer without inspecting prompts or responses.

## Cockpit views

### Portfolio

Show active passports, owners as opaque authorized references, business unit, declared criticality, policy version, current evaluation state, active profile, last re-evaluation, evidence freshness, and required actions. Personal or sensitive use-case descriptions are not copied into list telemetry.

### Evidence and policy

Show satisfied, failed, and unknown rule counts; stale/expiring sources; certification scope/validity; policy versions; exceptions; revocations; exact changed facts; and the material-change reconciliation state. No aggregate “compliance percentage” can hide a failed mandatory rule.

### Access and activity

Show requests and refusals by passport, profile, credential alias, configuration, operation, and environment; route/fallback selection; quota; errors; and incidents. Credentials, prompts, responses, documents, and person identifiers are absent.

### Remediation and decision-process quality

Show unassigned, overdue, escalated, accepted-risk, and evidence-closed actions by severity and accountable role. Show deterministic replay, explanation fidelity, evidence completeness/freshness, approval separation, and qualification-suite health. These measures describe control-process quality; they do not estimate output quality, benchmark models, or imply legal compliance.

### Performance and availability

Show latency p50/p95/p99, throughput, error/retry/fallback rates, availability, context-size distribution in bounded non-content units, capacity, and measurement window for already approved routes. Provider claims and observed measurements are visually distinct.

### Cost

Show sourced input/output/OCR/embedding/storage prices, minimum infrastructure cost, retries/fallbacks, actual spend, budget, and projection ranges with assumptions and price snapshot dates. False precision is forbidden. Cost observations inform accountable humans; they never order candidates, select a provider, compensate for a failed rule, or broaden access.

## Explicit non-ranking boundary

The locked product specification excludes benchmark leaderboards, a model-quality oracle, and procurement decision automation. MP-P6 therefore presents no “best model”, quality rank, recommendation score, sovereignty rank, or automated substitution proposal.

A future comparative capability would require an owner-reviewed specification and ADR amendment, a new accepted work package, task-specific corpus and methodology authority, security/privacy review, and separate acceptance gates. It cannot enter MP-P6 as a dashboard-only metric.

## Metric governance

[`../METRICS.md`](../METRICS.md) owns each metric's stable ID, definition/formula, kind, introduction phase, privacy class, and gate or diagnostic use. Each emitted metric-version record must additionally bind its unit, accountable owner, producer/source kind, observation window, freshness, retention, and missing-value behavior before the measure may be displayed. “All metrics” is not a valid collection rule: only actionable and qualified measures are retained.

A metric definition change creates a new version and does not rewrite old dashboards. Unknown data is displayed as unknown, never zero. Metric snapshots bind exact versions and inputs. Dashboard thresholds may alert but cannot issue policy, evaluation, profile, credential, or gateway commands.

## Data minimization and retention

Operational telemetry contains opaque organization/passport/profile/configuration/credential aliases and bounded numeric measures. It excludes content and secrets. Small groups and rare events require aggregation/suppression controls where they could identify a person indirectly. Any separate product-output evaluation corpus remains outside routine production telemetry and outside this phase.

## Non-goals

- ingesting prompts/responses by default;
- one opaque score combining security, compliance, operational health, and cost;
- benchmark leaderboard, model-quality oracle, or procurement recommendation;
- treating provider marketing benchmarks as observed truth;
- changing access because a metric or budget changed;
- claiming a standard or regulation is satisfied from a dashboard badge;
- unlimited-cardinality labels, unbounded exports, or N+1 model queries.

## Metrics

This phase operationalizes the catalogue entries needed for observation, especially `MP-MET-EVID-001`, `MP-MET-WATCH-003`, `MP-MET-WATCH-004`, `MP-MET-OPS-001`, `MP-MET-OPS-002`, `MP-MET-COST-001`, `MP-MET-ACCESS-001`, and `MP-MET-PII-001`.

## Exit gates

### MP-P6-G01 — The metric catalogue is versioned and complete

Every displayed measure has a stable definition, formula, unit, source, owner, freshness, privacy classification, retention, and missing-value behavior. Metric changes are versioned and historical views remain interpretable.

### MP-P6-G02 — Observations are structurally separated from authority

Contracts and tests prove cockpit queries and alert jobs have no policy, evaluation, profile, credential, route-selection, or gateway mutation capability. Failed/unknown rules cannot be offset, and cost or performance data cannot grant authorization.

### MP-P6-G03 — Evidence health and remediation are actionable

Evidence states, material-change reconciliation, deterministic replay, explanation fidelity, approval separation, action owner, due instant, escalation, disposition, and closure evidence are traceable to stable records. No control-process measure is labelled as model quality or compliance.

### MP-P6-G04 — Activity telemetry is attributable without content

Requests, refusals, routes, fallbacks, costs, performance, and incidents bind opaque passport/profile/consumer/configuration IDs. Automated tests and privacy review prove no prompt, response, document, secret, or direct person identifier reaches telemetry.

### MP-P6-G05 — Cost and performance measures are operationally sound

Observed versus provider-claimed data is distinct; latency percentiles, throughput, availability, retries, fallback costs, price dates, and projection assumptions are explicit. Queries are cursor-bounded, indexed, and free of N+1 behavior.

### MP-P6-G06 — Cockpit explanations are accessible and non-deceptive

Every badge has text, source, freshness, and scope. Failed, unknown, expired, unavailable, and not-applicable states are distinguishable without color. No percentage or certification badge implies global legal compliance, and no view implies a preferred model.

### MP-P6-G07 — Alerts and budgets cannot mutate authority

Budget, drift, incident, remediation, and threshold alerts are idempotent and actionable but cannot modify policy, passport, profile, credential, route, or verdict. Authorized commands remain required for every state change.

### MP-P6-G08 — Operational qualification and rollback pass

Load, cardinality, retention/deletion, backup/restore, dashboard outage, delayed metrics, corrupt series, provider outage, accessibility, security/privacy, and smoke tests pass on the release candidate. Missing telemetry degrades visibly and never weakens gateway enforcement.

## Dependencies and parallel work

MP-P6 depends on continuous re-evaluation and gateway event contracts. Privacy-safe telemetry, cost methodology, remediation workflows, and accessible dashboard design can progress in parallel only after `GOALS.md`/`STATUS.md` activation and an accepted work package plus owner-reviewed telemetry/specification/ADR amendments.

## Release and rollback

The cockpit is observational. Rolling it back cannot change policies, evaluations, profiles, credentials, or gateway enforcement. If metric processing is unavailable, the UI marks data stale or unavailable and stops affected alerts; it never substitutes zeros, hidden defaults, or a recommendation.
