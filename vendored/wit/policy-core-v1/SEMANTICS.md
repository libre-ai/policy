# policy-core-v1 — normative semantics

This document is normative for `libre-ai:policy-core@1.0.0`. The JSON Schemas and
`world.wit` define the accepted bytes and boundary; this document defines the only
valid evaluation, ordering and hashing behavior. The key words **MUST**, **MUST NOT**,
**SHOULD** and **MAY** have the meaning defined by RFC 2119.

## 1. Scope and safety boundary

`PolicyEvaluation` is an advisory eligibility statement for one approved policy,
one sourced model snapshot, one declared need and one explicit instant. `eligible`
means only “all rules are satisfied for these exact inputs”. It is not a ranking,
purchase recommendation, authorization, approval or instruction to transact.
The evaluator MUST NOT buy, select, deploy or approve anything and MUST NOT create
or mutate a policy approval. It verifies only that `approval.subjectDigest` binds
the policy subject; the authorized caller MUST establish the authenticity and
current validity of `approval.reference` before invocation.

The world is pure. It receives no clock, network, storage, identity, authorization
or randomness capability. The caller authorizes access before invoking it.

`engineVersion` MUST be an immutable SemVer constant embedded in the qualified
component. It MUST NOT be accepted from the caller or derived from any input. The
reference vectors use `1.0.0`; changing the embedded version changes the evaluation
content, ID and digest even when all policy inputs are unchanged.

## 2. Accepted values and validation order

Inputs are UTF-8 JSON objects conforming respectively to:

1. `policy-definition.v1.schema.json`;
2. `model-snapshot.v1.schema.json`;
3. `policy-need.v1.schema.json`.

The decoder MUST reject a BOM, invalid UTF-8, duplicate JSON object member names,
unpaired Unicode surrogates and non-JSON numbers. Every JSON number is interpreted
as an IEEE-754 binary64 value and is schema-bounded to the inclusive safe-integer
range `[-9007199254740991, 9007199254740991]`; fractional values in that range are
allowed. `-0` and `0` are equal. Strings are never case-folded, trimmed or Unicode-
normalized.

`evaluated-at`, every snapshot `source.retrievedAt` and every emitted `evaluatedAt`
MUST be a real Gregorian instant in the exact UTC-seconds form
`YYYY-MM-DDTHH:mm:ssZ`. Leap seconds and fractional seconds are rejected.

Validation occurs before rule evaluation, in this order:

1. decode and validate all three schemas, otherwise `policy.input_invalid`;
2. validate `evaluated-at`, otherwise `policy.evaluated_at_invalid`;
3. reject repeated rule IDs, otherwise `policy.rule_id_duplicate`;
4. verify the three input digests and `approval.subjectDigest`, otherwise
   `policy.digest_mismatch`;
5. require exact equality of policy, snapshot and need `tenantId`, otherwise
   `policy.tenant_mismatch`.

A failure returns `contract-error`, not a `PolicyEvaluation`. Error messages are
constant and MUST NOT contain input values:

| code | message |
| --- | --- |
| `policy.input_invalid` | `input does not conform to policy-core-v1` |
| `policy.evaluated_at_invalid` | `evaluated-at is not canonical UTC seconds` |
| `policy.rule_id_duplicate` | `policy contains duplicate rule ids` |
| `policy.digest_mismatch` | `input digest does not match canonical content` |
| `policy.tenant_mismatch` | `policy, snapshot and need tenants differ` |

## 3. Fact namespaces, cardinality and duplicates

A rule resolves its `fact` by exact name only:

- `need.*` names are looked up only in `PolicyNeed.facts`;
- `model.*` names are looked up only in `ModelSnapshot.facts`.

There is no fallback, alias, path traversal, case folding or derived fact. A name
may occur zero, one or several times. Exact duplicate fact objects are rejected by
the schemas (`uniqueItems: true`). Equal name/value pairs with distinct snapshot
sources are not exact duplicates and remain distinct occurrences.

- zero occurrences produce `unknown / policy.fact_absent`;
- one occurrence is evaluated directly;
- several occurrences are all evaluated (universal, not existential semantics),
  then reduced with `failed > unknown > satisfied`.

Thus no implementation may cherry-pick a satisfying occurrence. Input array order
never affects a rule status.

## 4. Operator and type matrix

A fact occurrence is always one scalar. Arrays are policy sets and are legal only
as the right-hand value of `in` and `not-in`.

| operator | rule `value` | occurrence | predicate |
| --- | --- | --- | --- |
| `equals` | string, number or boolean | same scalar type | `occurrence = value` |
| `not-equals` | string, number or boolean | same scalar type | `occurrence ≠ value` |
| `in` | non-empty homogeneous set of strings, numbers or booleans | same type as set items | some set item equals occurrence |
| `not-in` | non-empty homogeneous set of strings, numbers or booleans | same type as set items | no set item equals occurrence |
| `at-least` | number | number | `occurrence ≥ value` |
| `at-most` | number | number | `occurrence ≤ value` |

String equality is exact Unicode scalar-sequence equality. Boolean equality is
exact. Number equality and ordering use IEEE-754 binary64 comparisons; schemas
exclude NaN and infinities. Set order has no meaning. No coercion is permitted:
`"1"`, `1` and `true` are three different types. An occurrence with a disallowed
type is `unknown / policy.fact_type_mismatch`; negated operators MUST NOT turn a
type mismatch into satisfaction. A rule with a disallowed operator/value
combination is schema-invalid and returns `policy.input_invalid`.

## 5. Freshness

`maxSourceAgeDays` is legal only for a `model.*` rule. For each matched model fact,
let:

```text
ageSeconds = instant(evaluated-at) - instant(fact.source.retrievedAt)
maximumAgeSeconds = maxSourceAgeDays × 86400
```

The multiplication is exact integer arithmetic. The occurrence is fresh iff
`0 ≤ ageSeconds ≤ maximumAgeSeconds`; the upper boundary is inclusive.

- `ageSeconds < 0` gives `unknown / policy.source_from_future`, whether or not a
  maximum age is configured;
- if a maximum is configured and `ageSeconds > maximumAgeSeconds`, the occurrence
  gives `unknown / policy.snapshot_stale`;
- without a maximum, any non-future source age is accepted.

Freshness is checked before type and operator evaluation. Need facts have no source
age. The policy rule's own source date and snapshot `capturedAt` are not substituted
for the matched fact's `retrievedAt`.

## 6. Occurrence and rule evaluation

For each occurrence, in order:

1. apply the freshness rules when it is a model fact;
2. require the operator's occurrence type;
3. evaluate the predicate in section 4;
4. emit `satisfied / policy.rule_satisfied` when true, otherwise
   `failed / policy.rule_failed`.

A rule reduces its occurrence outcomes by status priority:

1. any `failed` → `failed / policy.rule_failed`;
2. otherwise any `unknown` → `unknown`, choosing the first present reason in this
   fixed order: `policy.source_from_future`, `policy.snapshot_stale`,
   `policy.fact_type_mismatch`, `policy.fact_absent`;
3. otherwise → `satisfied / policy.rule_satisfied`.

Exactly one `ruleResult` is emitted per policy rule. `ruleResults` MUST be sorted by
ascending raw ASCII `rule.id`; IDs are unique and schema-limited to ASCII. Policy
rule order, fact order and set-item order MUST NOT affect this output order.

## 7. Verdict

After every rule has a result:

1. if any result is `failed`, verdict is `ineligible`;
2. otherwise, if any result is `unknown` and at least one corresponding rule has
   `unknown: ineligible`, verdict is `ineligible`;
3. otherwise, if any result is `unknown`, verdict is `indeterminate`;
4. otherwise every result is `satisfied` and verdict is `eligible`.

The result status remains `unknown` when its rule maps unknown to `ineligible`; the
verdict records the disposition without falsifying the evidence state. Therefore:

- `eligible` has only satisfied results;
- `indeterminate` has at least one unknown result and no failed result;
- `ineligible` has at least one failed result, or at least one unknown result whose
  policy disposition is `ineligible`.

A failed rule has priority over all unknown rules. Unknown can never yield
`eligible`, and no UI or caller may override it to `eligible`.

## 8. Origin is not jurisdiction

The evaluator MUST NOT derive, copy or approximate a jurisdiction from model,
provider or company origin; hosting country; headquarters; source URI; model ID;
or any other fact. In particular, `model.provider.origin` does not satisfy a rule
on `model.hosting.jurisdiction`. If the exact jurisdiction fact is absent, the rule
is unknown. If origin and jurisdiction are both present and differ, each remains
an independent fact and only the exact rule name is read.

A source adapter that labels origin-derived data as jurisdiction violates this
contract and MUST be refused upstream as `policy.origin_jurisdiction_conflated`.
The pure evaluator itself performs no inference and cannot repair provenance.

## 9. Canonicalization and digests

`JCS(x)` is RFC 8785 JSON Canonicalization Scheme over the validated IEEE-754 value.
`H(label, x)` is lowercase hexadecimal SHA-256 over these exact bytes:

```text
UTF8(label) || 0x00 || JCS(normalize(x))
```

Normalization changes only arrays declared unordered by this section:

- policy `rules`: sort by ascending rule `id`;
- `in`/`not-in` rule sets: sort by ascending JCS scalar bytes;
- need facts: sort by `(name, type-rank, JCS(value))`;
- snapshot facts: sort by `(name, type-rank, JCS(value), JCS(source))`;
- evaluation `ruleResults`: already sorted by rule ID.

The type rank is `boolean = 0`, `number = 1`, `string = 2`. All comparisons above
are bytewise unsigned lexicographic comparisons. No other array is reordered.

Input digests are:

- policy: `H("libre-ai.policy-definition.v1", {schemaVersion, id, tenantId,
  version, status, rules})`; both `policy.digest` and
  `policy.approval.subjectDigest` MUST equal it;
- snapshot: `H("libre-ai.model-snapshot.v1", snapshot without digest)`;
- need: `H("libre-ai.policy-need.v1", need without digest)`.

After verdict and sorted rule results are known, construct the unsigned evaluation
with these fields and no others, in any object-member order:

```text
schemaVersion, tenantId, policyId, policyDigest, snapshotId, snapshotDigest,
needDigest, engineVersion, verdict, ruleResults, evaluatedAt
```

Then:

```text
evaluationDigest = H("libre-ai.policy-evaluation.v1", unsignedEvaluation)
id = "urn:libre-ai:evaluation:" || evaluationDigest
digest = evaluationDigest
```

The final object is the unsigned evaluation plus `id` and `digest`. The digest does
not include either field, avoiding recursion; the ID is content-addressed by the
same digest. Given the same semantic inputs, engine version and evaluated instant,
TypeScript and Rust MUST emit byte-identical JCS output, ID, digest, verdict and
ordered trace.

## 10. Conformance vectors

- `contracts/fixtures/policy-core-v1/operators.json` is the atomic operator/type
  corpus;
- `contracts/fixtures/policy-core-v1/golden.json` contains complete cross-runtime
  evaluations and contract-error vectors.

An implementation conforms only if every vector matches exactly. Implementations
MUST NOT rewrite expected vectors from their own output.
