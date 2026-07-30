import { describe, expect, test } from "bun:test";

import { validatePolicyNeed } from "./policy-need";

const FACT = { name: "need.tenure", value: "senior" } as const;

const VALID = {
  schemaVersion: "libre-ai.policy-need.v2",
  id: "urn:libre-ai:need:need-alpha",
  tenantId: "ten_aaaaaaaaaaaaaaaa",
  facts: [FACT],
  digest: "c".repeat(64),
} as const;

function raw(overrides: Record<string, unknown>): Record<string, unknown> {
  return { ...VALID, ...overrides };
}
function withFact(factOverrides: Record<string, unknown>): Record<string, unknown> {
  return raw({ facts: [{ ...FACT, ...factOverrides }] });
}

describe("validatePolicyNeed — accepts conformant needs", () => {
  test("a single string fact", () => {
    const result = validatePolicyNeed(VALID);
    expect(result.status).toBe("valid");
    if (result.status !== "valid") return;
    expect(result.value.facts).toHaveLength(1);
    expect(Object.isFrozen(result.value)).toBe(true);
    expect(Object.isFrozen(result.value.facts[0])).toBe(true);
  });

  test.each([
    ["a number value", { value: 5 }],
    ["a boolean value", { value: false }],
  ])("accepts %s", (_label, factOverride) => {
    expect(validatePolicyNeed(withFact(factOverride)).status).toBe("valid");
  });

  test("multiple distinct facts", () => {
    const facts = [FACT, { name: "need.jurisdiction", value: "eu" }];
    expect(validatePolicyNeed(raw({ facts })).status).toBe("valid");
  });
});

describe("validatePolicyNeed — malformed (fail-closed, two-state)", () => {
  test.each([
    ["unknown top-level key", { extra: 1 }],
    ["wrong schemaVersion", { schemaVersion: "libre-ai.policy-need.v1" }],
    ["id not a need urn", { id: "urn:libre-ai:snapshot:x" }],
    ["tenantId without ten_", { tenantId: "org-x" }],
    ["digest not sha256", { digest: "z".repeat(64) }],
    ["facts empty", { facts: [] }],
  ])("is malformed: %s", (_label, override) => {
    expect(validatePolicyNeed(raw(override))).toEqual({ status: "malformed" });
  });

  test.each([
    ["fact name not need-scoped", { name: "model.provenance" }],
    ["fact name malformed", { name: "need.BAD" }],
    ["fact value out of scale", { value: 9007199254740992 }],
    ["fact with a source key (not on a need)", { name: "need.tenure", value: "x", source: {} }],
  ])("is malformed via a bad fact: %s", (_label, factOverride) => {
    expect(validatePolicyNeed(withFact(factOverride))).toEqual({ status: "malformed" });
  });

  test("duplicate fact names", () => {
    expect(validatePolicyNeed(raw({ facts: [FACT, FACT] }))).toEqual({ status: "malformed" });
  });

  test("a non-object input is malformed", () => {
    expect(validatePolicyNeed(null)).toEqual({ status: "malformed" });
    expect(validatePolicyNeed([])).toEqual({ status: "malformed" });
  });
});
