# MP-P4 — Continuous monitoring and re-evaluation

## Outcome

Use-case owners and policy reviewers can follow how eligible, ineligible, and indeterminate deployment configurations change when sourced facts, policies, needs, evidence freshness, or engine versions change. Every change creates new evidence; historical decisions remain immutable.

## User promise

The product identifies why a configuration entered or left the eligible set, when the change became known, which use cases are affected, and what action is required. It never silently rewrites an old decision or silently broadens an approved model set.

## Change sources

Re-evaluation can be triggered by:

- a new model artifact or immutable version;
- a new or removed serving route;
- provider contractual-entity or subprocessor change;
- inference, storage, logs, backup, support, retention, or training change;
- source expiry, correction, revocation, or licence restriction;
- policy approval, revocation, or exception expiry;
- need revision;
- evaluator qualification or revocation;
- price, latency, availability, error/retry/fallback, or evidence-health snapshot refresh for an already approved route.

Eligibility triggers and operational-observation triggers remain distinct. These observations describe route operations or control-process evidence only: MP-P4 does not measure model-output quality, compare models, or infer a preferred route. A price change cannot change rule status unless price is an explicit policy fact.

## Snapshot pipeline

Source adapters fetch only authorized destinations, validate licence and payload boundaries, derive allowlisted facts, and stage a candidate snapshot. Validation detects source rollback, future timestamps, conflicting values, incomplete routes, and origin/jurisdiction conflation. Human or policy-defined approval accepts the immutable snapshot before it can affect production decisions.

Raw restricted payloads and credentials never enter evidence exports. Failed refreshes retain the prior snapshot for replay while freshness rules may make new evaluations indeterminate.

## Re-evaluation behavior

The coordinator selects every active passport whose policy or required configuration facts intersect the change, then evaluates against explicit versions and instant. Work is idempotent and bounded; duplicate events do not create divergent results. Failures are isolated and retryable without partial scope changes.

Changes are asymmetric at the monitoring boundary:

- `eligible → ineligible`: publish an immutable high-severity transition event immediately;
- `eligible → indeterminate`: publish an immutable fail-closed transition event;
- `ineligible/indeterminate → eligible`: publish only a candidate event requiring review before any later access broadening;
- operational metric change only: update observational evidence, never authorization;
- revoked engine/snapshot: refuse new evaluation and publish revocation state while preserving historical replay metadata.

MP-P4 never creates, mutates, restricts, or broadens an access profile and never handles traffic. MP-P5 is the sole planned enforcement boundary: it consumes exact versioned transition/revocation events and applies deny-by-default profile semantics.

## Timeline and notifications

Each timeline entry identifies old/new evaluation IDs, changed rule IDs, old/new sourced facts, event instant, detection instant, source, affected passports, and action state. A remediation action records accountable role, assignee reference, severity-specific due instant, escalation path, approved disposition, and closure evidence. Notifications contain no prompt, document, user-provided free text, or personal data.

Users can subscribe by role and severity. Delivery failure cannot suppress the in-product state. Acknowledgement neither changes eligibility nor closes the action; closure requires the declared evidence or an attributable accepted-risk disposition.

## Data, scale, and degraded mode

Re-evaluation uses cursor-bounded queues and idempotent commands. It avoids N+1 source retrieval by importing one immutable source snapshot then evaluating affected needs locally. Backpressure may delay non-security operational refreshes; revocation and hard-rule changes receive priority. If freshness or revocation authority is unavailable, MP-P4 reports an unavailable state and emits no permissive transition. Later authorization consumers must deny according to their own accepted contract.

## Non-goals

- scraping sources without licence or destination authorization;
- treating provider marketing pages as permanent truth;
- creating, mutating, restricting, or broadening an access profile;
- denying or routing runtime traffic before the MP-P5 enforcement boundary;
- silently adding a newly eligible route to an access profile;
- changing historical evaluation bytes;
- sending production content to test model availability;
- alerting through messages that disclose use-case or personal details.

## Metrics

Required metrics are `MP-MET-WATCH-001`, `MP-MET-WATCH-002`, `MP-MET-WATCH-003`, `MP-MET-WATCH-004`, `MP-MET-FRESH-001`, `MP-MET-SAF-001`, and `MP-MET-PII-001`.

## Exit gates

### MP-P4-G01 — Source adapters are allowlisted and provenance-safe

Every adapter has destination, licence, payload-size, parsing, fact allowlist, freshness, and rollback controls. SSRF, DNS rebinding, credentials, restricted redistribution, and origin/jurisdiction conflation have negative tests.

### MP-P4-G02 — Snapshot changes are immutable and reviewable

Candidate and accepted snapshots have content digests and structured diffs. Conflicts, future dates, incomplete routes, revoked sources, and source rollback fail closed. Acceptance never mutates an existing snapshot.

### MP-P4-G03 — Affected-use-case selection is complete and bounded

Tests prove every policy/fact dependency selects the relevant active passports without cross-organization leakage, unbounded scans, or N+1 retrieval. Duplicate events remain idempotent.

### MP-P4-G04 — Re-evaluation preserves deterministic evidence

Each result binds exact need, policy, snapshot, engine, and instant. Historical evaluations remain byte-identical; a change creates a new result and a structured old/new link.

### MP-P4-G05 — Transition events are asymmetric and fail closed

Revoked, newly ineligible, and newly indeterminate results emit exact, immutable, idempotent restriction events for authorization consumers. Newly eligible results emit candidate events only. Tests prove MP-P4 cannot mutate profiles or traffic; later MP-P5 tests prove enforcement.

### MP-P4-G06 — Timeline, remediation, and alerts are accountable

The cockpit shows exact changed facts/rules, freshness, severity, affected opaque IDs, accountable role, assignee, due instant, escalation, disposition, closure evidence, and acknowledgement. Material-change taxonomy and reconciliation are versioned and complete. Notification channels expose no personal or business content and failure cannot hide in-product state.

### MP-P4-G07 — Operational recovery is qualified

Queue backpressure, adapter outage, stale source, duplicate event, partial worker failure, database restore, and rollback journeys have automated evidence. Security revocations retain priority; an unavailable authority dependency makes MP-P4 publish an unavailable state and no permissive transition. The accepted authorization-consumer contract—not MP-P4—must prove denial of new decisions.

## Dependencies and parallel work

MP-P4 depends on organization policy lifecycle in MP-P3. Source-adapter qualification, re-evaluation selection, and timeline UI can proceed in parallel only after `GOALS.md`/`STATUS.md` activation and an accepted work package plus owner-reviewed event, source-adapter, threat-model, and specification amendments. The event contract must keep enforcement exclusively in MP-P5.

## Release and rollback

A monitoring release can be disabled without altering existing evaluations. Rollback stops new refresh/re-evaluation jobs, preserves queued event IDs and accepted snapshots, and resumes idempotently on the prior qualified version. It never emits a permissive transition for unavailable or revoked authority and has no capability to broaden access.
