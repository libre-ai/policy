# MP-P3 — Organization governance

## Outcome

An organization translates its doctrine into sourced, approved, versioned policy and applies it consistently to use-case passports. Policy authors, approvers, business owners, and reviewers have explicit, tenant-isolated responsibilities; exceptions are bounded and cannot silently weaken the baseline.

## User promise

A business team sees the approved constraints that apply to its use case rather than answering security or legal questions repeatedly. A security or privacy reviewer can inspect who proposed and approved each rule, what source supports it, what changed, and which evaluations are affected.

## Governance model

The organization owns its risk doctrine. Model Policy supplies deterministic authoring, validation, comparison, approval, evaluation, and evidence mechanics; it does not silently create legal policy.

Roles include:

- **policy editor:** proposes rules and source updates;
- **policy approver:** distinct human who accepts an immutable version;
- **business owner:** confirms use-case facts;
- **security reviewer:** challenges security requirements and exceptions;
- **privacy/legal adviser:** reviews relevant mappings and evidence without being assumed to approve every processing operation;
- **auditor:** reads immutable histories and replays decisions;
- **service curator:** may propose sourced model facts but cannot approve organization policy.

Role names are organization-configurable only through an accepted authorization contract. Tenant and operation checks remain mandatory.

## Policy composition

An effective policy is assembled deterministically from accepted layers, for example:

1. non-overridable platform safety rules;
2. organization baseline;
3. sector or business-unit additions;
4. environment constraints;
5. bounded use-case exception.

Precedence and conflict behavior must be explicit. An exception cannot deactivate a non-overridable rule. The effective policy and every deactivation are materialized and digest-bound before evaluation; runtime hidden defaults are forbidden.

## Exception lifecycle

An exception identifies the exact organization, need revision, affected rule IDs, justification code, approver, approval reference, effective instant, expiry, and compensating controls. Free-form justification that may contain personal data is stored separately or avoided; evaluation sees only bounded identifiers.

Expiry returns the use case to the baseline policy for subsequent consumers and emits an immutable lifecycle event carrying the bounded affected scope. MP-P4 alone consumes lifecycle events to select and re-evaluate affected needs; MP-P3 never selects needs or runs re-evaluation. An exception never edits historical results.

## Authoring journey

1. Import or create a draft from an accepted template.
2. Add sourced rules with explicit unknown disposition.
3. Validate fact names, types, operators, source licence, and freshness.
4. Compare the draft with the current approved version.
5. Submit the exact digest for independent approval.
6. Approve or refuse with an attributable bounded decision.
7. Publish the immutable version and emit an immutable lifecycle event with bounded affected-scope identifiers; MP-P4 performs any later selection and re-evaluation.

A proposer cannot approve the same subject digest. Services and assistants may propose but approval remains a human organization act.

## Data and authorization

Organization policy, needs, evaluations, and approval references are tenant-owned and protected by both application authorization and PostgreSQL RLS. Actor IDs are opaque. The customer keeps a role-at-time identity binding through its IdP or a signed pseudonym map so an authorized auditor can establish who approved without placing direct identities in routine logs or public exports. Binding access, retention, deletion, custody, and break-glass rules are explicit.

Audit logs contain opaque actor IDs, role-at-time references, revisions, rule IDs, timestamps, and outcomes—not policy free text containing personal content, credentials, prompts, or documents. Policy source citations follow the same sanitized public-source boundary as model facts. Private organization documents may be referenced through approved internal evidence mechanisms without being copied into public exports.

## Non-goals

- replacing an organization's legal counsel, DPO, or RSSI;
- a universal policy declared suitable for every organization;
- self-approval by an editor, agent, or managed-service operator;
- informal exceptions applied outside the effective policy digest;
- using a compliance score to override a failed rule;
- issuing runtime credentials.

## Metrics

Required metrics are `MP-MET-GOV-001`, `MP-MET-GOV-002`, `MP-MET-PROV-001`, `MP-MET-SAF-001`, and `MP-MET-PII-001`.

## Exit gates

### MP-P3-G01 — Policy roles and authorization are contract-bound

Commands and queries bind opaque actor, organization tenant, resource, operation, revision, and expiry. Biscuit authorization and RLS negatives prove no cross-organization read/write or self-approved policy path.

### MP-P3-G02 — Authoring and validation are complete

The UI and API support draft creation, sourced rule editing, unknown behavior, validation, and safe refusal messages. Invalid fact names, operator/type mismatches, unsupported sources, and stale revisions fail closed.

### MP-P3-G03 — Human approval is digest-bound and separated

Approval references the exact policy digest; proposer and approver differ; stale or revoked approval is refused. Diff and review journeys are accessible and show every rule addition, removal, source, and disposition change.

### MP-P3-G04 — Effective-policy composition is deterministic

Layer precedence, non-overridable rules, conflicts, and deactivations have locked semantics and golden vectors. The same accepted layers produce the same effective policy and digest independent of storage order.

### MP-P3-G05 — Exceptions are bounded, expiring, and auditable

Every exception is scoped to an organization and need/rule set, has approval and expiry, cannot weaken protected rules, and emits immutable lifecycle events on activation, expiry, revocation, or baseline change. MP-P4 is the only phase that consumes those events to select and re-evaluate affected needs.

### MP-P3-G06 — Policy lifecycle evidence is exportable

An authorized auditor can inspect versions, proposal/approval separation, role-at-time binding, diffs, effective-policy derivation, exceptions, revocations, and affected evaluation IDs without receiving secrets or personal content in routine exports.

### MP-P3-G07 — Organization governance passes role reviews

Business, Architecture, Security, and France/EU Privacy reviews approve the exact contracts and implementation candidate. Owner control is recorded before real organization data, approvals, or persistence are enabled.

## Dependencies and parallel work

MP-P3 requires the Phase 1 need and evaluation journey but does not depend on LLM assistance. Policy authoring, authorization/RLS, and accessible diff UI may progress in parallel only after `GOALS.md`/`STATUS.md` activation and an accepted work package plus owner-reviewed contract/specification/ADR amendments authorize exclusive write paths.

## Release and rollback

Rollback preserves every approved policy and evaluation contract version. A faulty policy version is revoked for new evaluations and replaced by a newly approved version; historical evidence remains replayable. Database rollback cannot restore an expired/deleted exception or bypass accepted retention rules.
