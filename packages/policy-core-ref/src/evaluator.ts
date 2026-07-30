// Policy evaluation engine. SEMANTICS.md §2-8: validation, operators, freshness, verdict.

import { digest, jcs } from "./jcs";
import { normalize } from "./normalize";
import { parseStrictJson } from "./strict-parser";
import type {
  ErrorCode,
  FactValue,
  JsonRecord,
  Operator,
  PolicyEvaluation,
  ReasonCode,
  RuleStatus,
  RuleValue,
  Verdict,
} from "./types";
import { StrictJsonError } from "./types";

const ENGINE_VERSION = "2.0.0";

interface InputLimits {
  policyInput: number;
  snapshotInput: number;
  needInput: number;
  evaluatedAt: number;
  successfulOutput: number;
}

// SEMANTICS.md §2: resource budgets (preflight checks before parsing)
const INPUT_LIMITS: InputLimits = {
  policyInput: 8 * 1024 * 1024,
  snapshotInput: 8 * 1024 * 1024,
  needInput: 8 * 1024 * 1024,
  evaluatedAt: 20,
  successfulOutput: 2 * 1024 * 1024,
};

export type EvaluateResult = { ok: true; jcs: Uint8Array } | { ok: false; error: ErrorCode };

export async function evaluate(
  policyBytes: Uint8Array,
  snapshotBytes: Uint8Array,
  needBytes: Uint8Array,
  evaluatedAt: string,
): Promise<EvaluateResult> {
  try {
    // SEMANTICS.md §2: byte length preflight
    if (
      policyBytes.byteLength > INPUT_LIMITS.policyInput ||
      snapshotBytes.byteLength > INPUT_LIMITS.snapshotInput ||
      needBytes.byteLength > INPUT_LIMITS.needInput ||
      new TextEncoder().encode(evaluatedAt).byteLength > INPUT_LIMITS.evaluatedAt
    ) {
      return { ok: false, error: "input-invalid" };
    }

    // Decode with strict JSON parser (depth 64, no duplicates, no BOM)
    const maxDepth = 64;
    const policy = parseStrictJson(policyBytes, maxDepth);
    const snapshot = parseStrictJson(snapshotBytes, maxDepth);
    const need = parseStrictJson(needBytes, maxDepth);

    // SEMANTICS.md §2.1: validate inputs in order
    const validation = validateInputs(policy, snapshot, need, evaluatedAt);
    if (validation !== null) {
      return { ok: false, error: validation };
    }

    // Compute digests for comparison (SEMANTICS.md §9)
    const policySubject = {
      schemaVersion: policy.schemaVersion,
      id: policy.id,
      tenantId: policy.tenantId,
      version: policy.version,
      status: policy.status,
      proposedBy: policy.proposedBy,
      rules: policy.rules,
    };
    const policyDigest = digest(
      "libre-ai.policy-definition.v2",
      normalize(policySubject, "policy"),
    );
    const snapshotDigest = digest(
      "libre-ai.model-snapshot.v2",
      normalize(without(snapshot, "digest"), "snapshot"),
    );
    const needDigest = digest(
      "libre-ai.policy-need.v2",
      normalize(without(need, "digest"), "need"),
    );

    // SEMANTICS.md §2.6: verify digests
    if (
      policy.digest !== policyDigest ||
      (policy.approval as JsonRecord).subjectDigest !== policyDigest ||
      snapshot.digest !== snapshotDigest ||
      need.digest !== needDigest
    ) {
      return { ok: false, error: "digest-mismatch" };
    }

    // SEMANTICS.md §4: validate all rule values against their operators
    const rules = policy.rules as JsonRecord[];
    if (Array.isArray(rules)) {
      for (const rule of rules) {
        const operator = String(rule.operator) as Operator;
        const value = rule.value as RuleValue;
        if (!isValidRuleValue(operator, value)) {
          return { ok: false, error: "input-invalid" };
        }
      }
    }

    // Evaluate all rules
    const ruleResults = evaluateRules(policy, snapshot, need, evaluatedAt);

    // Compute verdict (SEMANTICS.md §7)
    const verdict = computeVerdict(policy, ruleResults);

    // Build evaluation (SEMANTICS.md §9)
    const unsignedEvaluation: PolicyEvaluation = {
      schemaVersion: "libre-ai.policy-evaluation.v2",
      tenantId: String(policy.tenantId),
      policyId: String(policy.id),
      policyDigest: policyDigest,
      snapshotId: String(snapshot.id),
      snapshotDigest: snapshotDigest,
      needDigest: needDigest,
      engineVersion: ENGINE_VERSION,
      verdict: verdict,
      ruleResults: ruleResults,
      evaluatedAt: evaluatedAt,
      digest: "", // computed below
      id: "", // computed below
    };

    const evaluationDigest = digest(
      "libre-ai.policy-evaluation.v2",
      without(unsignedEvaluation as unknown as JsonRecord, "id", "digest"),
    );
    unsignedEvaluation.digest = evaluationDigest;
    unsignedEvaluation.id = `urn:libre-ai:evaluation:${evaluationDigest}`;

    // Serialize to JCS
    const resultJcs = jcs(unsignedEvaluation);

    // SEMANTICS.md §2: output ceiling check
    if (resultJcs.byteLength > INPUT_LIMITS.successfulOutput) {
      return { ok: false, error: "input-invalid" };
    }

    return { ok: true, jcs: resultJcs };
  } catch (error) {
    if (error instanceof StrictJsonError) {
      if (error.defect === "invalid-utf8") {
        return { ok: false, error: "input-invalid" };
      }
      return { ok: false, error: "input-invalid" };
    }
    // Unexpected error -> input-invalid (safe default)
    return { ok: false, error: "input-invalid" };
  }
}

function validateInputs(
  policy: JsonRecord,
  snapshot: JsonRecord,
  need: JsonRecord,
  evaluatedAt: string,
): ErrorCode | null {
  // SEMANTICS.md §2.4: validate evaluated-at format (YYYY-MM-DDTHH:mm:ssZ)
  if (!isUtcSeconds(evaluatedAt)) {
    return "evaluated-at-invalid";
  }

  // SEMANTICS.md §2.5: check for duplicate rule IDs
  const rules = policy.rules as JsonRecord[];
  if (Array.isArray(rules)) {
    const ruleIds = new Set<string>();
    for (const rule of rules) {
      const id = String(rule.id);
      if (ruleIds.has(id)) {
        return "rule-id-duplicate";
      }
      ruleIds.add(id);
    }
  }

  // SEMANTICS.md §3: check for exact duplicate facts (by JCS canonicalization)
  if (hasDuplicateFacts(snapshot)) {
    return "input-invalid";
  }
  if (hasDuplicateFacts(need)) {
    return "input-invalid";
  }

  // SEMANTICS.md §2.5: approval separation (approverId != proposedBy, actorKind = human)
  const approval = policy.approval as JsonRecord;
  if (approval.approverId === policy.proposedBy) {
    return "approval-invalid";
  }
  if (approval.actorKind !== "human") {
    return "approval-invalid";
  }

  // SEMANTICS.md §2.5: tenant mismatch
  if (policy.tenantId !== snapshot.tenantId || policy.tenantId !== need.tenantId) {
    return "tenant-mismatch";
  }

  return null;
}

function hasDuplicateFacts(container: JsonRecord): boolean {
  const facts = container.facts as JsonRecord[];
  if (!Array.isArray(facts)) {
    return false;
  }

  // Compute JCS for each fact and detect duplicates
  const jcsStrings = new Set<string>();
  for (const fact of facts) {
    const factJcs = jcs(fact);
    // Convert Uint8Array to string for Set comparison
    const factJcsStr = Array.from(factJcs).join(",");
    if (jcsStrings.has(factJcsStr)) {
      // Exact duplicate found (same name, value, AND source)
      return true;
    }
    jcsStrings.add(factJcsStr);
  }

  return false;
}

function evaluateRules(
  policy: JsonRecord,
  snapshot: JsonRecord,
  need: JsonRecord,
  evaluatedAt: string,
): Array<{ ruleId: string; status: RuleStatus; reasonCode: ReasonCode }> {
  const rules = policy.rules as JsonRecord[];
  const results: Array<{ ruleId: string; status: RuleStatus; reasonCode: ReasonCode }> = [];

  for (const rule of rules) {
    const ruleId = String(rule.id);
    const fact = String(rule.fact);
    const operator = String(rule.operator) as Operator;
    const value = rule.value as RuleValue;
    const maxSourceAgeDays =
      typeof rule.maxSourceAgeDays === "number" ? rule.maxSourceAgeDays : undefined;

    // Find fact objects (not just values) so we can access source for freshness
    let factObjects: JsonRecord[] = [];
    if (fact.startsWith("need.")) {
      factObjects = findFactObjects(need, fact);
    } else if (fact.startsWith("model.")) {
      factObjects = findFactObjects(snapshot, fact);
    }

    // SEMANTICS.md §3: zero occurrences -> unknown/fact_absent
    if (factObjects.length === 0) {
      results.push({
        ruleId,
        status: "unknown",
        reasonCode: "policy.fact_absent",
      });
      continue;
    }

    // SEMANTICS.md §3: multiple occurrences -> evaluate all, reduce
    const occurrenceStatuses: Array<{ status: RuleStatus; reasonCode: ReasonCode }> = [];
    for (const factObject of factObjects) {
      const occurrence = factObject.value as FactValue;
      const status = evaluateOccurrence(
        occurrence,
        operator,
        value,
        maxSourceAgeDays,
        factObject,
        evaluatedAt,
      );
      occurrenceStatuses.push(status);
    }

    // SEMANTICS.md §6: reduce by priority: failed > unknown > satisfied
    const ruleResult = reduceOccurrences(occurrenceStatuses);
    results.push({
      ruleId,
      status: ruleResult.status,
      reasonCode: ruleResult.reasonCode,
    });
  }

  // SEMANTICS.md §6: sort by rule ID (ascending raw ASCII)
  results.sort((a, b) => {
    const aId = new TextEncoder().encode(a.ruleId);
    const bId = new TextEncoder().encode(b.ruleId);
    return compareBytes(aId, bId);
  });

  return results;
}

function evaluateOccurrence(
  occurrence: FactValue,
  operator: Operator,
  value: RuleValue,
  maxSourceAgeDays: number | undefined,
  factObject: JsonRecord,
  evaluatedAt: string,
): { status: RuleStatus; reasonCode: ReasonCode } {
  // SEMANTICS.md §5: freshness check (for model facts with source)
  if (factObject.source !== undefined) {
    const freshness = checkFreshness(evaluatedAt, factObject, maxSourceAgeDays);
    if (freshness !== null) {
      return freshness;
    }
  }

  // SEMANTICS.md §4: operator evaluation
  return evaluateOperator(occurrence, operator, value);
}

function checkFreshness(
  evaluatedAt: string,
  factObject: JsonRecord,
  maxSourceAgeDays: number | undefined,
): { status: "unknown"; reasonCode: ReasonCode } | null {
  const source = factObject.source as JsonRecord;
  const retrievedAt = String(source.retrievedAt);

  const ageSeconds =
    Math.floor(new Date(evaluatedAt).getTime() / 1000) -
    Math.floor(new Date(retrievedAt).getTime() / 1000);

  // SEMANTICS.md §5: source from future
  if (ageSeconds < 0) {
    return { status: "unknown", reasonCode: "policy.source_from_future" };
  }

  // SEMANTICS.md §5: snapshot stale
  if (maxSourceAgeDays !== undefined) {
    const maximumAgeSeconds = maxSourceAgeDays * 86400;
    if (ageSeconds > maximumAgeSeconds) {
      return { status: "unknown", reasonCode: "policy.snapshot_stale" };
    }
  }

  return null;
}

function evaluateOperator(
  occurrence: FactValue,
  operator: Operator,
  value: RuleValue,
): { status: RuleStatus; reasonCode: ReasonCode } {
  // SEMANTICS.md §4: operator/type matrix and predicate evaluation
  try {
    switch (operator) {
      case "equals": {
        if (typeof occurrence !== typeof value) {
          return { status: "unknown", reasonCode: "policy.fact_type_mismatch" };
        }
        return occurrence === value
          ? { status: "satisfied", reasonCode: "policy.rule_satisfied" }
          : { status: "failed", reasonCode: "policy.rule_failed" };
      }
      case "not-equals": {
        if (typeof occurrence !== typeof value) {
          // Type mismatch on negated operator is unknown, not satisfied
          return { status: "unknown", reasonCode: "policy.fact_type_mismatch" };
        }
        return occurrence !== value
          ? { status: "satisfied", reasonCode: "policy.rule_satisfied" }
          : { status: "failed", reasonCode: "policy.rule_failed" };
      }
      case "in": {
        if (!Array.isArray(value)) {
          return { status: "failed", reasonCode: "policy.rule_failed" };
        }
        for (const item of value) {
          if (typeof occurrence === typeof item && occurrence === item) {
            return { status: "satisfied", reasonCode: "policy.rule_satisfied" };
          }
        }
        return { status: "failed", reasonCode: "policy.rule_failed" };
      }
      case "not-in": {
        if (!Array.isArray(value)) {
          return { status: "failed", reasonCode: "policy.rule_failed" };
        }
        for (const item of value) {
          if (typeof occurrence === typeof item && occurrence === item) {
            return { status: "failed", reasonCode: "policy.rule_failed" };
          }
        }
        return { status: "satisfied", reasonCode: "policy.rule_satisfied" };
      }
      case "at-least": {
        if (typeof occurrence !== "number" || typeof value !== "number") {
          return { status: "unknown", reasonCode: "policy.fact_type_mismatch" };
        }
        return occurrence >= value
          ? { status: "satisfied", reasonCode: "policy.rule_satisfied" }
          : { status: "failed", reasonCode: "policy.rule_failed" };
      }
      case "at-most": {
        if (typeof occurrence !== "number" || typeof value !== "number") {
          return { status: "unknown", reasonCode: "policy.fact_type_mismatch" };
        }
        return occurrence <= value
          ? { status: "satisfied", reasonCode: "policy.rule_satisfied" }
          : { status: "failed", reasonCode: "policy.rule_failed" };
      }
      default:
        return { status: "failed", reasonCode: "policy.rule_failed" };
    }
  } catch {
    return { status: "failed", reasonCode: "policy.rule_failed" };
  }
}

function reduceOccurrences(statuses: Array<{ status: RuleStatus; reasonCode: ReasonCode }>): {
  status: RuleStatus;
  reasonCode: ReasonCode;
} {
  // SEMANTICS.md §6: failed > unknown > satisfied
  for (const s of statuses) {
    if (s.status === "failed") {
      return s;
    }
  }

  // SEMANTICS.md §6: among unknowns, choose by fixed priority order (not input order)
  // This ensures deterministic output regardless of fact array order
  const reasonPriority = [
    "policy.source_from_future",
    "policy.snapshot_stale",
    "policy.fact_type_mismatch",
    "policy.fact_absent",
  ] as const;

  for (const reason of reasonPriority) {
    const hit = statuses.find((s) => s.status === "unknown" && s.reasonCode === reason);
    if (hit) return hit;
  }

  return { status: "satisfied", reasonCode: "policy.rule_satisfied" };
}

function computeVerdict(
  policy: JsonRecord,
  ruleResults: Array<{ ruleId: string; status: RuleStatus; reasonCode: ReasonCode }>,
): Verdict {
  // SEMANTICS.md §7: verdict logic
  const rules = policy.rules as JsonRecord[];
  const ruleUnknownDisposition = new Map<string, string>();

  for (const rule of rules) {
    const ruleId = String(rule.id);
    const unknown = String(rule.unknown);
    ruleUnknownDisposition.set(ruleId, unknown);
  }

  // Step 1: if any result is failed -> ineligible
  for (const result of ruleResults) {
    if (result.status === "failed") {
      return "ineligible";
    }
  }

  // Step 2: if any result is unknown AND that rule's unknown disposition is ineligible -> ineligible
  for (const result of ruleResults) {
    if (result.status === "unknown" && ruleUnknownDisposition.get(result.ruleId) === "ineligible") {
      return "ineligible";
    }
  }

  // Step 3: if any result is unknown -> indeterminate
  for (const result of ruleResults) {
    if (result.status === "unknown") {
      return "indeterminate";
    }
  }

  // Step 4: all satisfied -> eligible
  return "eligible";
}

function findFactObjects(container: JsonRecord, factName: string): JsonRecord[] {
  const facts = container.facts as JsonRecord[];
  const results: JsonRecord[] = [];

  if (Array.isArray(facts)) {
    for (const fact of facts) {
      // Fact names in the container already include the full prefix (e.g., "model.score", "need.foo")
      if (fact.name === factName) {
        results.push(fact);
      }
    }
  }

  return results;
}

function isUtcSeconds(value: string): boolean {
  // YYYY-MM-DDTHH:mm:ssZ format, exactly 20 bytes UTF-8
  if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/.test(value)) {
    return false;
  }
  if (new TextEncoder().encode(value).byteLength !== 20) {
    return false;
  }
  const parsed = new Date(value);
  // Verify round-trip: parsing and serializing should produce the same string
  return !Number.isNaN(parsed.getTime()) && parsed.toISOString() === `${value.slice(0, -1)}.000Z`;
}

function without(value: JsonRecord, ...keys: string[]): JsonRecord {
  const result: JsonRecord = {};
  const keySet = new Set(keys);
  for (const [k, v] of Object.entries(value)) {
    if (!keySet.has(k)) {
      result[k] = v;
    }
  }
  return result;
}

function compareBytes(left: Uint8Array, right: Uint8Array): number {
  for (let i = 0; i < Math.min(left.length, right.length); i++) {
    const diff = (left[i] ?? 0) - (right[i] ?? 0);
    if (diff !== 0) {
      return diff;
    }
  }
  return left.length - right.length;
}

/**
 * Validate that a rule value matches the constraints of its operator.
 * SEMANTICS.md §4: operator-value matrix defines allowed types.
 */
function isValidRuleValue(operator: Operator, value: RuleValue): boolean {
  if (operator === "equals" || operator === "not-equals") {
    // These operators require a scalar value (not an array)
    return typeof value === "string" || typeof value === "number" || typeof value === "boolean";
  }

  if (operator === "in" || operator === "not-in") {
    // These operators require a non-empty array value
    if (!Array.isArray(value) || value.length === 0) {
      return false;
    }
    // All elements must be the same type (homogeneous)
    const firstType = typeof value[0];
    for (const item of value) {
      if (typeof item !== firstType) {
        return false;
      }
      if (firstType !== "string" && firstType !== "number" && firstType !== "boolean") {
        return false;
      }
    }
    return true;
  }

  if (operator === "at-least" || operator === "at-most") {
    // These operators require a number value
    return typeof value === "number";
  }

  // Unknown operator (shouldn't happen if Operator type is correct)
  return false;
}
