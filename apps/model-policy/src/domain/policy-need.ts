// Model-policy domain — the pure validator for a policy need
// (docs/apps/model-policy.md; contracts/schemas/policy-need.v2.schema.json). A
// need is the third evaluation input: the set of `need.*` facts a policy is
// evaluated against. This module imports nothing, persists nothing, transmits
// nothing, and does NOT evaluate. Unlike a model snapshot, need facts carry no
// source, and the refusal matrix defines no authoring-time semantic refusal for
// a need's structure — so validation is two-state: `valid` or `malformed`. The
// evaluation-time codes (fact_absent, ...) and the cross-input tenant_mismatch
// belong to the deferred evaluator, not here. Patterns reuse the LOCKED
// common.v1 / policy-need.v2 $defs verbatim.

const NEED_ID = /^urn:libre-ai:need:[A-Za-z0-9._~-]+$/;
const TENANT_ID = /^ten_[a-z0-9]{16,64}$/;
const SHA256 = /^[a-f0-9]{64}$/;
const FACT_NAME = /^need\.[a-z][a-z0-9_.-]+$/;
const FACT_STRING = /^[A-Za-z0-9][A-Za-z0-9._:/+~-]{0,255}$/;

const MIN_SAFE = -9007199254740991;
const MAX_SAFE = 9007199254740991;

export type FactScalar = string | number | boolean;

export interface NeedFact {
  readonly name: string;
  readonly value: FactScalar;
}
export interface PolicyNeed {
  readonly schemaVersion: "libre-ai.policy-need.v2";
  readonly id: string;
  readonly tenantId: string;
  readonly facts: readonly NeedFact[];
  readonly digest: string;
}

export type NeedValidation =
  | { readonly status: "valid"; readonly value: PolicyNeed }
  | { readonly status: "malformed" };

const MALFORMED: NeedValidation = { status: "malformed" };

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
function hasExactKeys(obj: Record<string, unknown>, allowed: readonly string[]): boolean {
  const permitted = new Set(allowed);
  return Object.keys(obj).every((key) => permitted.has(key));
}
function isFactString(value: unknown): value is string {
  return typeof value === "string" && FACT_STRING.test(value);
}
function isSafeNumber(value: unknown): value is number {
  return typeof value === "number" && value >= MIN_SAFE && value <= MAX_SAFE;
}
function validScalar(value: unknown): value is FactScalar {
  return isFactString(value) || isSafeNumber(value) || typeof value === "boolean";
}

function validFact(value: unknown): NeedFact | undefined {
  if (!isObject(value) || !hasExactKeys(value, ["name", "value"])) return undefined;
  if (typeof value.name !== "string" || value.name.length > 128 || !FACT_NAME.test(value.name)) {
    return undefined;
  }
  if (!validScalar(value.value)) return undefined;
  return Object.freeze({ name: value.name, value: value.value });
}

const KEYS = ["schemaVersion", "id", "tenantId", "facts", "digest"] as const;

/**
 * Validate untrusted input as a policy need. Fail-closed and two-state: any
 * identity or fact-structure failure — unknown keys, malformed urn/tenant, a
 * fact whose name is not `need.*` or whose value is out of scale, a duplicate
 * fact name, an out-of-bounds facts array — is `malformed`. A conformant need is
 * `valid`. The matrix defines no authoring-time semantic refusal for a need.
 */
export function validatePolicyNeed(input: unknown): NeedValidation {
  if (!isObject(input) || !hasExactKeys(input, KEYS)) return MALFORMED;
  for (const key of KEYS) {
    if (!(key in input)) return MALFORMED;
  }
  if (input.schemaVersion !== "libre-ai.policy-need.v2") return MALFORMED;
  if (typeof input.id !== "string" || !NEED_ID.test(input.id)) return MALFORMED;
  if (typeof input.tenantId !== "string" || !TENANT_ID.test(input.tenantId)) return MALFORMED;
  if (typeof input.digest !== "string" || !SHA256.test(input.digest)) return MALFORMED;
  if (!Array.isArray(input.facts) || input.facts.length < 1 || input.facts.length > 1000)
    return MALFORMED;

  const facts: NeedFact[] = [];
  for (const raw of input.facts) {
    const fact = validFact(raw);
    if (fact === undefined) return MALFORMED;
    facts.push(fact);
  }
  // Fact names are the stable identity; a duplicate name is a non-unique need.
  if (new Set(facts.map((f) => f.name)).size !== facts.length) return MALFORMED;

  return {
    status: "valid",
    value: Object.freeze({
      schemaVersion: "libre-ai.policy-need.v2",
      id: input.id,
      tenantId: input.tenantId,
      facts: Object.freeze(facts),
      digest: input.digest,
    }),
  };
}
