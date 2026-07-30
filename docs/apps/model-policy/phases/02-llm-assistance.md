# MP-P2 — Optional LLM assistance

## Outcome

An optional assistant pre-fills and challenges the Phase 1 tunnel without becoming an authority. Disabling or losing the assistant leaves the deterministic journey, semantics, and eligibility result intact.

## User promise

A user may describe a use case in ordinary language and receive structured suggestions, contradiction warnings, and focused clarification prompts. Every suggestion is visibly provisional, attributable to its source passage, and individually accepted, edited, or rejected before it changes the passport.

## Authority boundary

The assistant can propose only values already accepted by the Phase 1 question/fact catalogue. It cannot:

- confirm an answer;
- add a fact directly to a bounded need;
- suppress or reorder a mandatory deterministic question;
- modify organization policy or model evidence;
- infer a jurisdiction from origin;
- lower sensitivity silently;
- convert unknown into eligible;
- select, authorize, buy, deploy, or route a model;
- generate the authoritative rule explanation.

The deterministic tunnel computes required questions. The pure evaluator receives only confirmed facts. AI output remains untrusted operational data and is never policy authority.

## Assistance journeys

### Initial pre-fill

The user enters a bounded description that excludes documents, credentials, and unnecessary personal data. The assistant emits schema-constrained field suggestions. Each suggestion carries the source excerpt, assistant route/version, and creation instant outside the pure need contract.

### Between-step challenge

After a deterministic step, the assistant may point out a likely contradiction or missing consideration, such as a financial extraction use case declaring no financial identifiers. The user returns to the relevant controlled question; the assistant does not create a new free-form fact.

### Explanation

The assistant may reformulate static product help, but the displayed verdict and rule reason come from stable codes and sourced facts. Generative prose is labelled as assistance and excluded from the evidence authority.

### Later document corroboration

Document-assisted extraction is a separate optional increment within this phase only after a dedicated privacy/security gate. Positive detections may propose more restrictive facts. Absence of detection never reduces sensitivity or proves corpus composition. Purpose, population, and consequences still require declaration.

## Assistant route

The pre-fill service itself must use a prequalified route suitable for the maximum sensitivity accepted by its input surface. It has no provider training, content retention, or content logging. If that route is unavailable or revoked, assistance fails closed and the no-LLM tunnel remains available.

The UI must not invite users to paste real documents before this boundary is approved. Client-side deterministic helpers and organization templates precede remote LLM use.

## Evaluation corpus

A locked corpus covers typical, atypical, multilingual, contradictory, adversarial, and injection-bearing descriptions. It contains synthetic or legally approved non-personal data. Evaluation measures each field independently and gives special weight to omissions or suggestions that would lower restrictions.

A small usability study can validate comprehension, not extraction safety. Promotion requires corpus evidence and adversarial testing.

## Data and retention

Raw assistant input and output are transient by default. Durable records retain only confirmed normalized facts and bounded operational metrics; no prompt, source excerpt containing personal data, or natural-person identifier enters logs or evaluation records. Any optional retained correction corpus requires separate consent/legal basis, minimization, deletion, and tenant controls.

## Metrics

Required metrics are `MP-MET-AI-001`, `MP-MET-AI-002`, `MP-MET-AI-003`, `MP-MET-SAF-001`, and `MP-MET-PII-001`. Acceptance rate is diagnostic and never a safety target.

## Exit gates

### MP-P2-G01 — Suggestions conform to the tunnel schema

All assistant outputs pass a strict bounded schema and reference existing question/fact identifiers. Unknown fields, prose-only answers, malformed values, and oversized output are refused without changing the passport.

### MP-P2-G02 — Confirmation remains mandatory

Browser and domain tests prove no assistant response reaches the bounded need before explicit user confirmation. Disabling, timing out, or changing the assistant produces no eligibility side effect.

### MP-P2-G03 — Every suggestion has visible provenance

The interface shows that a value is AI-suggested, the bounded source excerpt that motivated it, and accept/edit/reject controls. Assistive technology receives the same distinction; color is not the sole cue.

### MP-P2-G04 — Shadow evaluation meets the safety gate

On the locked corpus, field precision/recall and correction metrics are published, every eligibility-impacting discrepancy is surfaced before confirmation, and no silent omission or risk-lowering suggestion can produce an eligible result.

### MP-P2-G05 — Prompt injection and hostile content are contained

Adversarial descriptions and optional document text cannot alter system policy, mandatory questions, tool permissions, source citations, or final rule explanations. Untrusted text remains structurally isolated from authority.

### MP-P2-G06 — The assistant route is independently qualified

Security/privacy review verifies hosting, entity, retention, training, subprocessors, encryption, observability, revocation, and degraded mode for the assistant itself. No external unqualified provider receives tunnel content.

### MP-P2-G07 — The no-LLM journey remains equivalent

With assistance disabled, unavailable, or revoked, all Phase 1 journeys and evidence remain usable. Confirmed identical facts produce byte-identical evaluations regardless of how they were entered.

## Dependencies and parallel work

MP-P2 is a conditional extension that depends on the accepted Phase 1 fact and question catalogue. It is not required by MP-P3–MP-P7 or by the base managed service; a customer contract can omit it entirely. Corpus construction, assistant adapter design, and labelled interaction design can proceed in parallel only after an accepted work package and owner-reviewed contract/threat-model amendments authorize the capability. No user data or network capability is enabled before the assistant-route and privacy gates.

## Release and rollback

The assistant is feature-gated and removable without migrating authoritative needs or evaluations. Rollback disables pre-fill and challenge endpoints, expires transient data, and returns users to the deterministic tunnel. Confirmed passports remain valid because they contain no dependency on model-generated prose.
