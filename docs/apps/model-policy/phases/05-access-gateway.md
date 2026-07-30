# MP-P5 — Access profiles and gateway

## Outcome

An approved evaluation can be transformed, through a separate authorized command, into a versioned access profile enforced by a gateway. Consumers use individually attributable credentials; provider secrets remain server-side. Revocation and newly unsafe evidence restrict access without waiting for manual key replacement.

## User promise

A team shares one logical use-case access profile and evidence page, not one shared secret. Each application and environment receives its own revocable credential while all consume only the model routes approved for that profile.

## Boundary from eligibility

`PolicyEvaluation.eligible` remains an advisory statement for exact inputs. It is not authorization. Access requires a separate aggregate and approval that binds:

- organization tenant;
- passport revision and need digest;
- policy, snapshot, and evaluation IDs/digests;
- permitted deployment-configuration IDs;
- permitted operation/modalities;
- environment and consumer identity;
- quotas and budget bounds;
- effective/expiry instants;
- approved fallbacks;
- logging, retention, and training restrictions;
- approver and approval reference.

The pure evaluator never receives a credential or calls the gateway.

## Profile lifecycle

Commands create a draft profile, attach eligible routes, submit the exact digest, approve, activate, restrict, revoke, and expire it. Optimistic revision and idempotency are mandatory. A proposer cannot approve the same profile.

The default managed behavior is asymmetric:

- a route that becomes ineligible, indeterminate, stale, or revoked is removed from new traffic;
- an already approved eligible fallback may take over;
- a newly eligible route is visible but not added automatically;
- cost, latency, or other observational changes do not modify access;
- a policy/need change requires a new profile revision and approval.

A fully pinned mode may forbid route substitution. Both modes are explicit in the profile; no hidden runtime default exists.

## Credentials

The product distinguishes:

- **passport:** declared business need;
- **access profile:** approved logical permissions;
- **credential:** secret or capability held by one consumer;
- **decision record:** shareable non-secret evidence.

External API compatibility may use opaque high-entropy credentials stored only as protected verifiers. Internal service calls use attenuated, short-lived Biscuit capabilities. The exact format requires a locked contract and threat model. Provider API keys are never returned to users, browsers, exports, logs, or model-policy core.

A credential is scoped to one organization, profile, application/service, and environment. Rotation and revocation are independent. Shared human credentials are forbidden.

## Gateway enforcement

The gateway validates credential status, tenant/profile binding, operation, quota, route eligibility/revocation generation, the exact versioned MP-P4 transition authority, and request bounds before selecting an approved route. It sends only the minimum required payload to the selected provider and applies provider-specific retention/log controls where technically enforceable.

The gateway cannot prove that arbitrary request content matches the declared passport merely from a key. Higher-sensitivity profiles require a dedicated endpoint/workspace, bounded modalities, optional approved content controls, and contractual/user accountability. The product must state this residual risk rather than claim complete runtime compliance.

## Audit and privacy

Audit events contain opaque credential/profile/configuration IDs, operation, timestamps, outcome, latency and bounded usage units. They contain no secret, prompt, response, document, natural-person identifier, or provider authorization header. Diagnostic content capture is off by default and outside this phase.

## Non-goals

- distributing or displaying provider secrets;
- one secret shared by a team;
- auto-authorizing every newly eligible model;
- allowing cockpit observations to override profile scope;
- claiming a credential proves every request matches the business declaration;
- direct provider fallback outside the gateway;
- production enablement before key ceremony, penetration, rollback, and owner gates.

## Metrics

Required metrics are `MP-MET-ACCESS-001`, `MP-MET-ACCESS-002`, `MP-MET-ACCESS-003`, `MP-MET-SAF-001`, and `MP-MET-PII-001`.

## Exit gates

### MP-P5-G01 — Access-profile contracts separate evaluation from authorization

Commands, events, schemas, and state transitions bind exact evaluation evidence, organization, routes, operations, environment, quotas, expiry, fallbacks, and approval. No evaluator result directly grants access.

### MP-P5-G02 — Profile approval and tenant authorization are fail closed

Proposal/approval separation, idempotency, optimistic revision, Biscuit policy, revocation, expiry, and PostgreSQL RLS pass cross-organization and privilege-escalation tests.

### MP-P5-G03 — Credentials are individual, scoped, and recoverable

Each consumer/environment receives a distinct credential with rotation, expiry, emergency revocation, leak response, and last-used metadata. Only protected verifier material persists; plaintext is shown at most once through a secure boundary.

### MP-P5-G04 — Provider secrets never cross the gateway boundary

Secret scanning, browser tests, export tests, logs, errors, traces, backups, and support workflows prove provider credentials and authorization headers are neither exposed nor reflected.

### MP-P5-G05 — Runtime route enforcement follows asymmetric change rules

Ineligible, indeterminate, stale, revoked, expired, and out-of-profile routes are denied. Approved fallback works deterministically. Newly eligible routes and cockpit observations cannot broaden the profile without approval.

### MP-P5-G06 — Gateway data minimization is qualified

Adapters transmit only required request fields to qualified routes, enforce bounded sizes/timeouts, and avoid content logging. Provider training/retention controls and their enforcement limits are documented per route.

### MP-P5-G07 — Runtime misuse and content residual risk are explicit

The product documents what the profile can enforce and what remains declarative. Dedicated high-sensitivity surfaces and optional content controls are tested without claiming perfect classification of arbitrary inputs.

### MP-P5-G08 — Security, performance, and recovery gates pass

Threat model, key ceremony, rotation, revocation propagation, load/latency, provider outage, fallback, quota race, replay, backup/restore, incident response, penetration, smoke, and rollback evidence are approved before production activation.

## Dependencies and parallel work

MP-P5 depends on accepted governance and monitoring semantics. Profile contracts and gateway adapters can progress in parallel only after `GOALS.md`/`STATUS.md` activation, an accepted work package, and owner-reviewed authorization, revocation-event, credential, gateway, threat-model, and ADR amendments lock the boundary. Key handling and production enablement require independent security and owner controls.

## Release and rollback

Rollback denies creation or broadening of profiles, preserves current revocations, and routes only through the previous qualified gateway/profile interpreter. If safe interpretation is unavailable, traffic stops rather than falling back directly to a provider. Credential and provider-key rotation follows the incident runbook; historical decision records contain only non-secret identifiers.
