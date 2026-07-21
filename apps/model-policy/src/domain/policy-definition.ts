// Model-policy domain — the pure validator for an approved PolicyDefinition
// (docs/apps/model-policy.md; contracts/schemas/policy-definition.v2.schema.json).
// TypeScript owns authoring and human approval; the deterministic rule EVALUATOR
// is the deferred Rust/WASM boundary — this module implements no evaluation. It
// validates untrusted input into a typed, contract-conformant PolicyDefinition
// and reports, fail-closed, WHY it is not acceptable, distinguishing:
//   - `malformed` — not a well-formed policy at all (identity/structure fails the
//     schema); a boundary concern, not a domain refusal.
//   - `refused`   — well-formed but violating a domain invariant, with the exact
//     policy.* matrix code: a non-approved status → version_unapproved; a rule
//     using an unsupported operation, an operator/value-type mismatch, or a
//     source-age bound on a non-model fact → rule_unbounded.
//   - `valid`     — a typed, conformant approved policy.
// Patterns reuse the LOCKED common.v1 / policy-definition.v2 $defs verbatim.

import {
  type FactScalar,
  isFactString,
  isObject,
  isSafeNumber,
  validScalar,
} from "./fact-primitives";

export type { FactScalar };

const POLICY_ID = /^urn:libre-ai:policy:[A-Za-z0-9._~-]+$/;
const TENANT_ID = /^ten_[a-z0-9]{16,64}$/;
const IDENTIFIER = /^[a-z][a-z0-9_-]{2,127}$/;
const URN = /^urn:libre-ai:[a-z][a-z0-9-]*:[A-Za-z0-9._~-]+$/;
const SHA256 = /^[a-f0-9]{64}$/;
const PRINCIPAL_ID = /^(?:usr|svc)_[a-z0-9]{16,64}$/;
const USER_ID = /^usr_[a-z0-9]{16,64}$/;
const FACT_NAME = /^(?:model|need)\.[a-z][a-z0-9_.-]+$/;
// The policySource.uri pattern from policy-definition.v2, reused verbatim. This
// is authoring-time SHAPE validation only; destination safety (rejecting private,
// loopback or metadata hosts, DNS-rebinding) is a FETCH-TIME concern owned by the
// deferred source adapter — cf. radar's destination policy — not this validator,
// which stays faithful to the locked contract rather than stricter than it.
const HTTPS_URI =
  /^https:\/\/(?:[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?\.)+[A-Za-z]{2,63}(?::[0-9]{1,5})?(?:\/[A-Za-z0-9._~/-]*)?$/;
// The source timestamp pattern is UTC seconds (policy-definition.v2 policySource).
const UTC_SECONDS = /^[0-9]{4}-[0-9]{2}-[0-9]{2}T(?:[01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]Z$/;
// approvedAt is common.v1 timestamp (`format: date-time`). RFC 3339 §5.6 requires
// a time offset (Z or ±HH:MM), so requiring one here is contract-faithful, not
// stricter than the schema.
const TIMESTAMP = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/;

const OPERATORS = ["equals", "not-equals", "in", "not-in", "at-least", "at-most"] as const;
const SCALAR_OPERATORS = ["equals", "not-equals"] as const;
const SET_OPERATORS = ["in", "not-in"] as const;
const NUMERIC_OPERATORS = ["at-least", "at-most"] as const;
const UNKNOWN_MODES = ["indeterminate", "ineligible"] as const;

// version is bounded by the JS safe-integer max (schema: integer, minimum 1).
const MAX_SAFE = 9007199254740991;

export type Operator = (typeof OPERATORS)[number];
// A value is a scalar or a homogeneous set; homogeneity is a runtime invariant
// enforced by `validSet`, not expressed at the type level.
export type FactValue = FactScalar | readonly FactScalar[];

export interface PolicySource {
  readonly uri: string;
  readonly retrievedAt: string;
  readonly digest: string;
  readonly licence: string;
}
export interface Rule {
  readonly id: string;
  readonly fact: string;
  readonly operator: Operator;
  readonly value: FactValue;
  readonly unknown: (typeof UNKNOWN_MODES)[number];
  readonly source: PolicySource;
  readonly maxSourceAgeDays?: number;
}
export interface Approval {
  readonly role: "policy-approver";
  readonly actorKind: "human";
  readonly approverId: string;
  readonly approvedAt: string;
  readonly reference: string;
  readonly subjectDigest: string;
}
export interface PolicyDefinition {
  readonly schemaVersion: "libre-ai.policy-definition.v2";
  readonly id: string;
  readonly tenantId: string;
  readonly version: number;
  readonly status: "approved";
  readonly proposedBy: string;
  readonly rules: readonly Rule[];
  readonly digest: string;
  readonly approvedAt: string;
  readonly approval: Approval;
}

export type PolicyRefusalCode = "policy.version_unapproved" | "policy.rule_unbounded";

export type PolicyValidation =
  | { readonly status: "valid"; readonly value: PolicyDefinition }
  | { readonly status: "malformed" }
  | { readonly status: "refused"; readonly refusal: PolicyRefusalCode };

const MALFORMED: PolicyValidation = { status: "malformed" };
function refused(refusal: PolicyRefusalCode): PolicyValidation {
  return { status: "refused", refusal };
}

function hasKeys(
  obj: Record<string, unknown>,
  required: readonly string[],
  optional: readonly string[],
): boolean {
  const allowed = new Set([...required, ...optional]);
  if (Object.keys(obj).some((key) => !allowed.has(key))) return false;
  return required.every((key) => key in obj);
}
function isInt(value: unknown, min: number, max: number): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= min && value <= max;
}

function validSource(value: unknown): PolicySource | undefined {
  if (!isObject(value) || !hasKeys(value, ["uri", "retrievedAt", "digest", "licence"], []))
    return undefined;
  if (typeof value.uri !== "string" || value.uri.length > 2048 || !HTTPS_URI.test(value.uri))
    return undefined;
  if (typeof value.retrievedAt !== "string" || !UTC_SECONDS.test(value.retrievedAt))
    return undefined;
  if (typeof value.digest !== "string" || !SHA256.test(value.digest)) return undefined;
  if (typeof value.licence !== "string" || value.licence.length < 1 || value.licence.length > 128) {
    return undefined;
  }
  return Object.freeze({
    uri: value.uri,
    retrievedAt: value.retrievedAt,
    digest: value.digest,
    licence: value.licence,
  });
}

// A set value must be a 1..100 unique array of a single scalar kind (booleans
// cap at 2). Deterministic membership requires a homogeneous, de-duplicated set.
function validSet(value: unknown): value is readonly FactScalar[] {
  if (!Array.isArray(value) || value.length < 1) return false;
  if (value.every((v) => typeof v === "boolean")) {
    return value.length <= 2 && new Set(value).size === value.length;
  }
  if (value.every(isFactString)) {
    return value.length <= 100 && new Set(value).size === value.length;
  }
  if (value.every(isSafeNumber)) {
    return value.length <= 100 && new Set(value).size === value.length;
  }
  return false;
}

// Result of validating a single rule: a structurally-bad rule is malformed; a
// well-formed rule whose operation is unsupported or inconsistent is unbounded.
type RuleOutcome =
  | { readonly kind: "ok"; readonly rule: Rule }
  | { readonly kind: "malformed" }
  | { readonly kind: "unbounded" };

function validRule(value: unknown): RuleOutcome {
  if (
    !isObject(value) ||
    !hasKeys(value, ["id", "fact", "operator", "value", "unknown", "source"], ["maxSourceAgeDays"])
  ) {
    return { kind: "malformed" };
  }
  if (typeof value.id !== "string" || !IDENTIFIER.test(value.id)) return { kind: "malformed" };
  if (typeof value.fact !== "string" || value.fact.length > 128 || !FACT_NAME.test(value.fact)) {
    return { kind: "malformed" };
  }
  if (
    typeof value.unknown !== "string" ||
    !(UNKNOWN_MODES as readonly string[]).includes(value.unknown)
  ) {
    return { kind: "malformed" };
  }
  const source = validSource(value.source);
  if (source === undefined) return { kind: "malformed" };
  if (value.maxSourceAgeDays !== undefined && !isInt(value.maxSourceAgeDays, 1, 3650)) {
    return { kind: "malformed" };
  }

  // An operator outside the supported set is an unsupported operation.
  if (
    typeof value.operator !== "string" ||
    !(OPERATORS as readonly string[]).includes(value.operator)
  ) {
    return { kind: "unbounded" };
  }
  const operator = value.operator as Operator;

  // The value type must match the operator, and a source-age bound is only
  // meaningful on a model fact — otherwise the operation is unbounded.
  if ((SCALAR_OPERATORS as readonly string[]).includes(operator)) {
    if (!validScalar(value.value)) return { kind: "unbounded" };
  } else if ((SET_OPERATORS as readonly string[]).includes(operator)) {
    if (!validSet(value.value)) return { kind: "unbounded" };
  } else if ((NUMERIC_OPERATORS as readonly string[]).includes(operator)) {
    if (!isSafeNumber(value.value)) return { kind: "unbounded" };
  }
  if (value.maxSourceAgeDays !== undefined && !value.fact.startsWith("model.")) {
    return { kind: "unbounded" };
  }

  const rule: Rule = {
    id: value.id,
    fact: value.fact,
    operator,
    value: Array.isArray(value.value)
      ? Object.freeze([...(value.value as FactScalar[])])
      : (value.value as FactScalar),
    unknown: value.unknown as Rule["unknown"],
    source,
    ...(value.maxSourceAgeDays === undefined ? {} : { maxSourceAgeDays: value.maxSourceAgeDays }),
  };
  return { kind: "ok", rule: Object.freeze(rule) };
}

function validApproval(value: unknown): Approval | undefined {
  if (
    !isObject(value) ||
    !hasKeys(
      value,
      ["role", "actorKind", "approverId", "approvedAt", "reference", "subjectDigest"],
      [],
    )
  ) {
    return undefined;
  }
  if (value.role !== "policy-approver" || value.actorKind !== "human") return undefined;
  if (typeof value.approverId !== "string" || !USER_ID.test(value.approverId)) return undefined;
  if (typeof value.approvedAt !== "string" || !UTC_SECONDS.test(value.approvedAt)) return undefined;
  if (typeof value.reference !== "string" || !URN.test(value.reference)) return undefined;
  if (typeof value.subjectDigest !== "string" || !SHA256.test(value.subjectDigest))
    return undefined;
  return Object.freeze({
    role: "policy-approver",
    actorKind: "human",
    approverId: value.approverId,
    approvedAt: value.approvedAt,
    reference: value.reference,
    subjectDigest: value.subjectDigest,
  });
}

const KEYS = [
  "schemaVersion",
  "id",
  "tenantId",
  "version",
  "status",
  "proposedBy",
  "rules",
  "digest",
  "approvedAt",
  "approval",
] as const;

/**
 * Validate untrusted input as an approved PolicyDefinition. Identity/structural
 * failures return `malformed`; a non-`approved` status returns
 * `policy.version_unapproved`; a rule using an unsupported operator, a value that
 * does not match its operator, or a source-age bound on a non-model fact returns
 * `policy.rule_unbounded`. The rule EVALUATOR is not implemented here.
 */
export function validatePolicyDefinition(input: unknown): PolicyValidation {
  if (!isObject(input) || !hasKeys(input, KEYS, [])) return MALFORMED;
  if (input.schemaVersion !== "libre-ai.policy-definition.v2") return MALFORMED;
  if (typeof input.id !== "string" || !POLICY_ID.test(input.id)) return MALFORMED;
  if (typeof input.tenantId !== "string" || !TENANT_ID.test(input.tenantId)) return MALFORMED;
  if (!isInt(input.version, 1, MAX_SAFE)) return MALFORMED;
  if (typeof input.proposedBy !== "string" || !PRINCIPAL_ID.test(input.proposedBy))
    return MALFORMED;
  if (typeof input.digest !== "string" || !SHA256.test(input.digest)) return MALFORMED;
  if (typeof input.approvedAt !== "string" || !TIMESTAMP.test(input.approvedAt)) return MALFORMED;
  const approval = validApproval(input.approval);
  if (approval === undefined) return MALFORMED;

  // A non-string status is a structural failure (malformed); a well-typed status
  // that is not "approved" is a draft/unapproved policy (a domain refusal).
  if (typeof input.status !== "string") return MALFORMED;
  if (input.status !== "approved") return refused("policy.version_unapproved");

  if (!Array.isArray(input.rules) || input.rules.length < 1 || input.rules.length > 1000)
    return MALFORMED;
  const rules: Rule[] = [];
  for (const raw of input.rules) {
    const outcome = validRule(raw);
    if (outcome.kind === "malformed") return MALFORMED;
    if (outcome.kind === "unbounded") return refused("policy.rule_unbounded");
    rules.push(outcome.rule);
  }
  // rules are uniqueItems in the schema — a duplicate rule id is not a valid set.
  // Rule ids are the stable identity; a duplicate id is an ambiguous, non-unique
  // rule set (stricter than the schema's object-level uniqueItems, on purpose).
  if (new Set(rules.map((r) => r.id)).size !== rules.length) return MALFORMED;

  return {
    status: "valid",
    value: Object.freeze({
      schemaVersion: "libre-ai.policy-definition.v2",
      id: input.id,
      tenantId: input.tenantId,
      version: input.version,
      status: "approved",
      proposedBy: input.proposedBy,
      rules: Object.freeze(rules),
      digest: input.digest,
      approvedAt: input.approvedAt,
      approval,
    }),
  };
}
