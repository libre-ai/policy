# MP-P1 — Deterministic qualification without an LLM

## Outcome

A business user completes an explained, rule-driven tunnel and obtains a deterministic list of deployment configurations that are eligible, ineligible, or indeterminate for the declared use case. The entire journey works without an LLM and remains the permanent authority and fallback for later phases.

## User promise

The user is asked only for facts within their assigned responsibility. Data, application, contract, organization-policy, and provider facts come from their respective accountable authorities. The result explains which configurations satisfy the evaluated policy, which fail, what remains unknown, and why; it never says that eligibility alone authorizes use.

## Actors

- **Business declarant:** describes the actual task, intended purpose, affected people, and consequences.
- **Use-case owner:** approves the exact passport digest and remains accountable for renewal or revocation.
- **Data owner/steward:** supplies or approves data categories, special-category/criminal-data handling, and re-identification facts when the declarant cannot.
- **Application/service owner:** supplies topology, volume, latency, availability, fallback, and integration facts.
- **Procurement/contract owner:** supplies provider entity, contractual, retention, training, support, and subprocessor evidence.
- **Policy approver:** owns the applicable approved policy, not each questionnaire answer.
- **Security/privacy reader:** inspects rule traces and unresolved evidence.

Every field definition names its accountable actor, accepted evidence source, allowed delegate, and unknown/escalation path. Conflicting answers remain unresolved until the accountable actor decides; the most permissive answer is never selected automatically. The DPO advises on privacy requirements but is not silently modelled as the universal processing approver.

## Tunnel

The question order and branching are versioned data evaluated by deterministic rules. An answer can reveal a fixed next question; no generative component selects or suppresses mandatory questions.

### Step 1 — Purpose and expected result

Capture the business task, intended purpose, intended users, output type, human review, whether an output informs or triggers a decision, and consequences of error. Do not ask the user to classify the AI Act regime.

### Step 2 — People and impact

Capture affected groups, direct or indirect identification, vulnerable people, possible adverse effects, and contestability. Do not ask the user to make a legal conclusion.

### Step 3 — Input and output data

Capture confidentiality, personal-data presence, special categories, criminal data, financial identifiers, trade secrets, intellectual property, and re-identification risk separately. The data owner/steward confirms facts outside the declarant's knowledge. Public, personal, and critical are orthogonal dimensions.

### Step 4 — Processing lifecycle

Display applicable organization constraints and sourced provider/contract facts. Ask the declarant only for intended processing; contract owners confirm whether data may be retained, appear in logs, train a provider, enter support workflows, or cross specified locations. Inference, storage, logs, backups, and support are separate paths.

### Step 5 — Operational requirements

Capture modality, languages, representative input size, volume, latency, throughput, availability, output structure, acceptance criteria, budget ceiling, and fallback need. The application/service owner confirms topology and service-level facts; the procurement/contract owner confirms commercial constraints. These facts do not compensate for failed security rules and do not create a model-quality oracle.

### Step 6 — Review and confirmation

Present the normalized passport and exact digest before evaluation. Each fact shows its meaning, accountable actor, answer source, and unresolved conflict. “I do not know” remains an explicit unknown. The declarant can correct facts, but the use-case owner must attest the final digest; lowering a material restriction requires the designated policy/data/security approval. Renewal, revocation, and correction create attributable new revisions rather than rewriting the attested passport.

## Evaluation and result

The same confirmed need, policy, configuration snapshots, engine version, and evaluation instant produce the same results and ordering. Phase 1 sorts by verdict and stable identifier; it does not introduce an opaque recommendation score.

Each card represents a deployment configuration and shows:

- model artifact and version;
- provider route and contractual entity;
- inference, storage, log, backup, and support locations;
- retention and training behavior;
- `eligible`, `ineligible`, or `indeterminate` verdict;
- every rule result and sourced fact;
- evidence freshness;
- assumptions and unresolved facts.

A composite OCR/model/fallback path is eligible only if every mandatory node and data-flow edge satisfies the policy.

## Visual requirements and standards

The interface distinguishes:

1. legal/regulatory requirements;
2. certifications and attestations;
3. organization policy rules.

Rule status is `satisfied`, `failed`, `unknown`, or `not applicable`; colors never carry meaning alone. “Blocked by hosting location” appears only when a specific processing-location fact causes a rule failure. ISO or similar certification is shown as valid, expired, out of scope, or unevidenced—not as globally “respected” by a region.

## Decision record

The export contains need, policy, snapshot, engine and evaluation identifiers/digests, rule trace, source dates, unresolved facts, the use-case-owner attestation bound to the passport digest, and separate policy/data/access approval references. JSON canonical data is authoritative; HTML/PDF is a human projection. It contains no secret, document sample, natural-person identity, or raw questionnaire free text.

## Non-goals

- LLM suggestions or document analysis;
- automatic legal advice or blanket compliance claims;
- ranking by public leaderboard;
- credential issuance or provider traffic;
- silently accepting an incomplete need;
- forcing a fixed maximum number of questions at the expense of correctness.

## Accessibility and degraded mode

The tunnel, summary, rule trace, and sources are usable by keyboard and assistive technology without JavaScript-only meaning. Browser back/forward preserves confirmed answers safely. A source refresh outage permits replay of accepted snapshots but marks their age; it cannot make an unknown configuration eligible.

## Metrics

Required metrics are `MP-MET-TUN-001`, `MP-MET-TUN-002`, `MP-MET-DET-001`, `MP-MET-SAF-001`, `MP-MET-EXPL-001`, and `MP-MET-PII-001`.

## Exit gates

### MP-P1-G01 — Every question maps to an accepted fact

The question catalogue identifies per-field accountable actor, accepted evidence source, allowed delegate, reason, controlled values, derived follow-ups, unknown/escalation and conflict behavior, and exact `need.*` mapping. No question asks a business user to assert unsupported legal, contract, provider, or application-topology facts.

### MP-P1-G02 — Branching is deterministic and exhaustively tested

For every allowed answer state, the next required questions and completion state are stable and covered. Browser refresh, backtracking, answer replacement, and incompatible combinations cannot skip a mandatory fact.

### MP-P1-G03 — The passport confirmation boundary is explicit

Proposed, confirmed, unknown, delegated, disputed, and organization-supplied values are distinguishable. Evaluation receives only a bounded need whose exact digest is attested by the use-case owner. Corrections, renewal, and revocation are attributable; data, policy, privacy/security, and later access-profile approvals remain separate from that attestation.

### MP-P1-G04 — All deployment configurations receive a fail-closed verdict

Every candidate route is evaluated against the exact policy and evidence snapshot. Missing, stale, malformed, cross-tenant, and revoked inputs cannot produce an eligible card.

### MP-P1-G05 — Hosting and assurance failures are accurately visualized

Inference, storage, logs, backups, support, contractual entity, and subprocessors are separate. Legal rules, organization constraints, and certification evidence are not conflated. Each failure links to a rule and source.

### MP-P1-G06 — The decision record is portable and replayable

An independent actor can verify digests, replay the evaluation with the qualified engine, and obtain the same rule trace. Human projections match canonical data and contain no credential or personal content.

### MP-P1-G07 — The complete journey is accessible and privacy-safe

Three-engine browser evidence covers creation, unknown answers, corrections, evaluation, excluded/indeterminate inspection, export, keyboard use, screen-reader structure, and no-content logging. Cross-organization negatives are included.

### MP-P1-G08 — Business, security, and privacy validation passes

Representative non-expert users understand the questions and reasons; specialized reviewers confirm that the output does not overclaim legal compliance. Usability evidence is not substituted for correctness tests.

## Dependencies and parallel work

The question catalogue and accessible tunnel can progress in parallel with result-card and export design after MP-P0 fact names and evaluation contracts are accepted. Activation remains blocked until `GOALS.md` records owner selection, `STATUS.md` records wave 4b, and an accepted work package plus any required contract/specification amendments authorize the exact write paths. The phase cannot close before MP-P0 closes.

## Release and rollback

Phase 1 can release without any LLM or provider inference capability. A defective tunnel version is withdrawn and existing passports retain their questionnaire-version identity. Re-evaluation after a fix creates a new record; previous evidence remains visible and is never rewritten.
