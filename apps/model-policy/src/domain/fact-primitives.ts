// Shared fact/validation primitives for the model-policy domain. The three
// contract validators (policy-definition, model-snapshot, policy-need) all speak
// the same fact-scalar vocabulary from the locked schemas (common.v1 /
// policy-definition.v2 / model-snapshot.v2 / policy-need.v2). With three call
// sites the rule-of-three threshold is reached, so these primitives are extracted
// to a single source of truth rather than reimplemented per module.

// A scalar fact value: a bounded string, a safe-integer-range number, or a
// boolean (the factScalar $def shared across the policy schemas).
export type FactScalar = string | number | boolean;

// factScalar string pattern, verbatim from the locked schemas.
const FACT_STRING = /^[A-Za-z0-9][A-Za-z0-9._:/+~-]{0,255}$/;

const MIN_SAFE = -9007199254740991;
const MAX_SAFE = 9007199254740991;

export function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** True if `obj` has no key outside `allowed` (enforces additionalProperties:false). */
export function hasExactKeys(obj: Record<string, unknown>, allowed: readonly string[]): boolean {
  const permitted = new Set(allowed);
  return Object.keys(obj).every((key) => permitted.has(key));
}

export function isFactString(value: unknown): value is string {
  return typeof value === "string" && FACT_STRING.test(value);
}

export function isSafeNumber(value: unknown): value is number {
  return typeof value === "number" && value >= MIN_SAFE && value <= MAX_SAFE;
}

export function validScalar(value: unknown): value is FactScalar {
  return isFactString(value) || isSafeNumber(value) || typeof value === "boolean";
}
