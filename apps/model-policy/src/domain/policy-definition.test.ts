import { describe, expect, test } from "bun:test";

import { validatePolicyDefinition } from "./policy-definition";

const SOURCE = {
  uri: "https://example.org/policy",
  retrievedAt: "2026-07-21T10:00:00Z",
  digest: "a".repeat(64),
  licence: "CC-BY-4.0",
} as const;

const RULE = {
  id: "rule-one",
  fact: "model.provenance",
  operator: "equals",
  value: "verified",
  unknown: "ineligible",
  source: SOURCE,
} as const;

const VALID = {
  schemaVersion: "libre-ai.policy-definition.v2",
  id: "urn:libre-ai:policy:pol-alpha",
  tenantId: "ten_aaaaaaaaaaaaaaaa",
  version: 1,
  status: "approved",
  proposedBy: "svc_aaaaaaaaaaaaaaaa",
  rules: [RULE],
  digest: "b".repeat(64),
  approvedAt: "2026-07-21T12:00:00Z",
  approval: {
    role: "policy-approver",
    actorKind: "human",
    approverId: "usr_bbbbbbbbbbbbbbbb",
    approvedAt: "2026-07-21T11:00:00Z",
    reference: "urn:libre-ai:review:r-1",
    subjectDigest: "c".repeat(64),
  },
} as const;

function raw(overrides: Record<string, unknown>): Record<string, unknown> {
  return { ...VALID, ...overrides };
}
function withRule(ruleOverrides: Record<string, unknown>): Record<string, unknown> {
  return raw({ rules: [{ ...RULE, ...ruleOverrides }] });
}

describe("validatePolicyDefinition — accepts conformant approved policies", () => {
  test("a scalar-equals rule", () => {
    const result = validatePolicyDefinition(VALID);
    expect(result.status).toBe("valid");
    if (result.status !== "valid") return;
    expect(result.value.rules).toHaveLength(1);
    expect(Object.isFrozen(result.value)).toBe(true);
    expect(Object.isFrozen(result.value.rules[0])).toBe(true);
  });

  test.each([
    ["in with a string set", { operator: "in", value: ["verified", "attested"] }],
    ["not-in with a number set", { operator: "not-in", value: [1, 2, 3] }],
    ["at-least with a number", { operator: "at-least", value: 5, fact: "model.score" }],
    ["equals with a boolean", { operator: "equals", value: true }],
    ["a model fact with a source-age bound", { maxSourceAgeDays: 30 }],
  ])("accepts %s", (_label, ruleOverride) => {
    expect(validatePolicyDefinition(withRule(ruleOverride)).status).toBe("valid");
  });
});

describe("validatePolicyDefinition — version_unapproved", () => {
  test.each(["draft", "proposed", "superseded"])("refuses status %s", (status) => {
    expect(validatePolicyDefinition(raw({ status }))).toEqual({
      status: "refused",
      refusal: "policy.version_unapproved",
    });
  });
});

describe("validatePolicyDefinition — rule_unbounded", () => {
  test.each([
    ["an unsupported operator", { operator: "greater-than" }],
    ["in with a scalar value", { operator: "in", value: "verified" }],
    ["equals with an array value", { operator: "equals", value: ["a", "b"] }],
    ["at-least with a non-number", { operator: "at-least", value: "high" }],
    ["at-most with a boolean", { operator: "at-most", value: true }],
    ["a source-age bound on a need fact", { fact: "need.tenure", maxSourceAgeDays: 30 }],
  ])("refuses %s", (_label, ruleOverride) => {
    expect(validatePolicyDefinition(withRule(ruleOverride))).toEqual({
      status: "refused",
      refusal: "policy.rule_unbounded",
    });
  });
});

describe("validatePolicyDefinition — malformed (structural / identity)", () => {
  test.each([
    ["unknown top-level key", { extra: 1 }],
    ["wrong schemaVersion", { schemaVersion: "libre-ai.policy-definition.v1" }],
    ["id not a policy urn", { id: "urn:libre-ai:spec:x" }],
    ["tenantId without ten_", { tenantId: "org-x" }],
    ["version below 1", { version: 0 }],
    ["proposedBy malformed", { proposedBy: "adm_aaaaaaaaaaaaaaaa" }],
    ["digest not sha256", { digest: "z".repeat(64) }],
    ["approvedAt malformed", { approvedAt: "2026-07-21" }],
    ["rules empty", { rules: [] }],
  ])("is malformed: %s", (_label, override) => {
    expect(validatePolicyDefinition(raw(override))).toEqual({ status: "malformed" });
  });

  test.each([
    ["rule id malformed", { id: "Rule-One" }],
    ["fact not model/need scoped", { fact: "other.thing" }],
    ["unknown mode invalid", { unknown: "maybe" }],
    ["source uri not https", { source: { ...SOURCE, uri: "http://example.org/x" } }],
    [
      "source retrievedAt not UTC seconds",
      { source: { ...SOURCE, retrievedAt: "2026-07-21T10:00:00.000Z" } },
    ],
    ["maxSourceAgeDays out of range", { maxSourceAgeDays: 4000 }],
  ])("is malformed via a bad rule: %s", (_label, ruleOverride) => {
    expect(validatePolicyDefinition(withRule(ruleOverride))).toEqual({ status: "malformed" });
  });

  test("malformed approval", () => {
    expect(
      validatePolicyDefinition(raw({ approval: { ...VALID.approval, role: "auditor" } })),
    ).toEqual({ status: "malformed" });
  });

  test("duplicate rule ids", () => {
    expect(validatePolicyDefinition(raw({ rules: [RULE, RULE] }))).toEqual({ status: "malformed" });
  });

  test("a non-object input is malformed", () => {
    expect(validatePolicyDefinition(null)).toEqual({ status: "malformed" });
  });

  test.each([
    { status: 123 },
    { status: null },
    { status: ["approved"] },
  ])("a non-string status is malformed, not a domain refusal: %o", (override) => {
    expect(validatePolicyDefinition(raw(override))).toEqual({ status: "malformed" });
  });
});

describe("validatePolicyDefinition — source uri stays contract-faithful", () => {
  // Destination safety (private/loopback rejection) is the deferred fetch
  // adapter's job; the authoring validator accepts any schema-valid https URI.
  test("a private-TLD https source is accepted here (gated at fetch time, not authoring)", () => {
    expect(
      validatePolicyDefinition(withRule({ source: { ...SOURCE, uri: "https://internal.local/p" } }))
        .status,
    ).toBe("valid");
  });
});
