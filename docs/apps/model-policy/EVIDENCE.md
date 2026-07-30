# Model Policy evidence policy

## Purpose

Define what a phase gate may claim and where its evidence lives. This document does not make review reports normative: product specifications, contracts, policies, snapshots, needs, and evaluation semantics remain their own authorities. Evidence proves an assertion against an immutable candidate.

## Evidence levels

| Level | Meaning | Minimum evidence |
| --- | --- | --- |
| `declared` | Intent is documented, with no implementation claim | Accepted scope and non-goals |
| `implemented` | Code or content exists on an immutable commit | Commit and changed-path inventory |
| `verified` | Required behavior is reproducible | Commands, environment/tool versions, explicit results, negative cases |
| `qualified` | Release-relevant safety and quality have independent verdicts | Verified evidence plus role-separated business/architecture/security/privacy/accessibility/performance reviews as applicable |
| `in_service` | Behavior is observed on an authorized operated instance | Qualified release, structured deployment authorization, deployment identity, passing smoke/rollback observations, bounded operational window, and resolved incidents |

A gate definition states its `requiredEvidenceLevel`. Each phase also declares `requiredIndependentReviewRoles`; every `qualified` or `in_service` record for one of its gates must bind one distinct non-producer reviewer reference and one content-addressed review attestation for every declared role. The planning record never declares a gate or phase complete: owner-controlled execution authorities may do so only after every mandatory gate references evidence at or above that level.

## Evidence record requirements

Each gate reference points to one JSON record under `distribution/evidence/model-policy/` and pins the SHA-256 digest of that complete record. The record conforms to [`evidence-record.v1.schema.json`](evidence-record.v1.schema.json) and includes:

- stable evidence ID plus exact phase and gate IDs;
- SHA-256 of the canonical gate section (UTF-8, LF-normalized, from its exact `### <gate-id>` heading through the next heading of level 1–3);
- assertion under test and achieved evidence level;
- immutable source commit and relevant artifact/contract digests;
- non-empty commands and exact tool versions (never `latest`, `unknown`, or `unversioned`);
- content-digested repository fixtures, synthetic/legally usable corpus identities, or operated-environment identities;
- explicit expected and observed results;
- blocking, major, minor, and residual findings;
- an approving verdict for `qualified` and `in_service` claims, with no unresolved blocking or major finding;
- role-separated bindings from required role and opaque reviewer reference to a content-addressed review attestation;
- exposed harness identifiers for `verified`, `qualified`, and `in_service` claims, never reviewer PII;
- deployment identity also bound as an operated-environment input, a content-addressed tracked deployment-authorization artifact, a bounded observation window ending no later than the record creation instant, smoke/rollback artifacts under the operational evidence root, and incident state plus incident artifacts when any occurred for `in_service` claims; every operational artifact is distinct JSON conforming to [`operational-evidence.v1.schema.json`](operational-evidence.v1.schema.json), bound to the evidence/phase/gate/deployment/window, and carries only the expected favorable outcome;
- rollback or invalidation conditions;
- creation instant in ISO 8601 UTC.

The checker verifies the record digest, schema, phase/gate binding, achieved level, source commit, repository-fixture digest, role separation, every phase-required review role, approving verdict, service-observation bindings, and digests of source-commit regular-file artifacts. For each qualified role it also verifies a tracked regular non-symlink attestation against [`review-attestation.v1.schema.json`](review-attestation.v1.schema.json), exact candidate/phase/gate/role/reviewer binding, an approving attestation verdict with no blocking or major finding, and the digest of its human review report. The roadmap and every checker schema are read and parsed from regular Git-index blobs before validation. Evidence records, attestations, review reports, and in-service operational artifacts are likewise hashed, sensitive-marker scanned, and parsed from their regular Git-index blobs so one immutable byte buffer controls each check. Every canonical JSON Git object is size-preflighted before blob loading, limited to 1,048,576 bytes, and decoded as fatal UTF-8 before scanning or parsing; duplicate member names are then rejected before materialization, and JSON keys and string values are scanned again after decoding so malformed bytes, discarded members, or Unicode escapes cannot conceal a marker. Parser, schema-compiler, and schema-validation diagnostics expose stable categories only, never rejected names, values, patterns, or formats. Candidate artifacts and repository fixtures are read from exact source-commit blobs. Operational records must have distinct paths and IDs, match the evidence, phase, gate, deployment, and authorized window, carry `authorized`, `passed`, or `resolved` according to their role, record deployment authorization strictly before the window starts, keep observations inside it, and use coherent observation/record instants. Every evidence record must digest its phase document and bind the canonical gate-section digest both at the source commit and in the current Git-index phase-document blob. Unstaged mutable text cannot substitute for an indexed authority, while any staged semantic gate change invalidates older evidence; the CLI success summary is derived from that validated indexed roadmap rather than rereading the worktree. A bare path—even to an existing review—cannot pass a gate. A mutable branch output, self-review, shared reviewer reference across required roles, empty or unrelated operational JSON, unqualified screenshot, “tests passed” without commands and pinned tools, or provider marketing claim is insufficient.

Opaque reviewer references make role separation mechanically checkable without publishing identity. They do not prove human identity by themselves: the owner-controlled acceptance authority must resolve them through the retained role-at-time identity/attestation mechanism before recording promotion.

## Storage

- Normative product phase records live under `docs/apps/model-policy/`.
- Immutable supporting review dossiers live under `docs/reviews/`; they are not direct gate evidence.
- Content-addressed gate records, review attestations, and reproducible release evidence live under `distribution/evidence/model-policy/`; attestations live in its `reviews/` subtree.
- Restricted datasets and customer-instance evidence do not enter the public repository.
- GitHub comments may supplement evidence only when immutable URL/body digest is recorded.

Evidence filenames include phase/gate identity and a date or immutable candidate identity. Existing records are not overwritten; a superseding record links to the prior one.

## Business review

Business review verifies:

- questions are answerable by the assigned actor;
- terminology reflects real work rather than legal/technical jargon;
- outputs distinguish fact, inference, unknown, and recommendation;
- use-case consequences and human oversight are captured;
- the product does not overpromise legal compliance or model quality;
- business metrics and trade-offs are understandable and actionable.

Usability testing proves comprehension only. It cannot prove regulatory correctness, extraction safety, or deterministic semantics.

## Technical review

Technical review verifies:

- strict contracts and stable IDs;
- deterministic and fail-closed behavior;
- malformed, stale, revoked, duplicate, and cross-tenant negatives;
- resource bounds, pagination, indexes, and no N+1 paths;
- no secrets or PII in logs/evidence;
- tests at unit, contract, integration, browser, security, performance, smoke, and rollback layers as applicable;
- supply-chain and licence gates.

## Architecture review

Architecture review verifies:

- one authority per subject;
- pure eligibility, approval/authorization, gateway, and telemetry boundaries remain separate;
- ownership and tenant boundaries are explicit;
- phase dependencies are acyclic and do not smuggle later capabilities into earlier phases;
- events and state transitions are attributable and replayable;
- degradation and rollback preserve deny-by-default semantics;
- new cross-module behavior has an ADR or accepted contract amendment.

## Security and privacy review

A phase that adds natural-language assistance, source fetching, organization persistence, credentials, provider traffic, telemetry, or managed operations requires dedicated security and France/EU privacy review before activation. Review covers prompt injection, SSRF, tenant isolation, authorization, secret lifecycle, data minimization, lawful evidence boundaries, retention/deletion, subprocessors, location/jurisdiction, support access, and incident response.

No report may claim that EU hosting alone proves sovereignty, that company/model origin proves jurisdiction, or that one evidence pack certifies global GDPR/AI Act/ISO compliance.

## Human control

The repository owner retains explicit controls required by doctrine and contracts, including authority lock/amendment, first security-critical pattern, real organization/personal data, source-provider network capability, policy approvals, key ceremony, production, infrastructure, public/commercial exposure, and deployment.

Agent review and green CI are evidence; they do not replace a required owner milestone.

## Invalidation

A review becomes stale when any reviewed normative file, contract/vector digest, gate criterion, metric definition, security boundary, or release candidate changes. Editorial corrections that cannot affect meaning may be documented as non-invalidating only by the review protocol; otherwise re-review is required.

Revocation does not delete historical evidence. It records why an artifact cannot support new decisions and which replacement, if any, is current.

## Evidence export to customers

The product decision record may contain policy/need/snapshot/evaluation/profile IDs and digests, rule statuses, source citations/dates, approvals, configuration graph, metric snapshots, and revocation state. It MUST NOT contain provider/customer secrets, raw prompts/responses/documents, direct person identifiers, or unrestricted source payloads.

Where an auditor must identify a human approver, the customer retains a role-at-time identity binding through its IdP or a signed pseudonym map with explicit access, retention, deletion, and custody rules. Public and routine exports keep only opaque actor IDs; Model Policy does not make those IDs probative by itself.

Canonical machine data is the replay authority; PDF/HTML is a readable projection. Stronger external probative claims require accepted signature/attestation, trustworthy timestamp, custody, retention, access history, identity-resolution procedure, and legal-language review appropriate to the claim.
