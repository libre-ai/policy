import { describe, expect, test } from "bun:test";

import { validateModelSnapshot } from "./model-snapshot";

const SOURCE = {
  uri: "https://example.org/fact",
  retrievedAt: "2026-07-21T09:00:00Z",
  digest: "a".repeat(64),
  licence: "CC-BY-4.0",
} as const;

const FACT = { name: "model.provenance", value: "audited", source: SOURCE } as const;

const VALID = {
  schemaVersion: "libre-ai.model-snapshot.v2",
  id: "urn:libre-ai:snapshot:snap-alpha",
  tenantId: "ten_aaaaaaaaaaaaaaaa",
  modelId: "mdl_bbbbbbbbbbbbbbbb",
  capturedAt: "2026-07-21T10:00:00Z",
  facts: [FACT],
  digest: "c".repeat(64),
} as const;

function raw(overrides: Record<string, unknown>): Record<string, unknown> {
  return { ...VALID, ...overrides };
}
function withFact(factOverrides: Record<string, unknown>): Record<string, unknown> {
  return raw({ facts: [{ ...FACT, ...factOverrides }] });
}

describe("validateModelSnapshot — accepts conformant snapshots", () => {
  test("a single sourced string fact", () => {
    const result = validateModelSnapshot(VALID);
    expect(result.status).toBe("valid");
    if (result.status !== "valid") return;
    expect(result.value.facts).toHaveLength(1);
    expect(Object.isFrozen(result.value)).toBe(true);
    expect(Object.isFrozen(result.value.facts[0])).toBe(true);
    expect(Object.isFrozen(result.value.facts[0]?.source)).toBe(true);
  });

  test.each([
    ["a number value", { value: 42 }],
    ["a boolean value", { value: true }],
    ["a negative-safe number", { value: -9007199254740991 }],
  ])("accepts %s", (_label, factOverride) => {
    expect(validateModelSnapshot(withFact(factOverride)).status).toBe("valid");
  });

  test("multiple distinct facts", () => {
    const facts = [FACT, { name: "model.training-cutoff", value: "2026-01", source: SOURCE }];
    expect(validateModelSnapshot(raw({ facts })).status).toBe("valid");
  });
});

describe("validateModelSnapshot — snapshot_unsourced", () => {
  test("a fact missing its source", () => {
    const raw = withFact({});
    // biome-ignore lint/performance/noDelete: test constructs a fact without a source key
    delete (raw.facts as { source?: unknown }[])[0]?.source;
    expect(validateModelSnapshot(raw)).toEqual({
      status: "refused",
      refusal: "policy.snapshot_unsourced",
    });
  });

  test.each([
    ["source uri not https", { source: { ...SOURCE, uri: "http://example.org/x" } }],
    [
      "source missing digest",
      { source: { uri: SOURCE.uri, retrievedAt: SOURCE.retrievedAt, licence: "CC0" } },
    ],
    [
      "source retrievedAt not UTC seconds",
      { source: { ...SOURCE, retrievedAt: "2026-07-21T09:00:00.000Z" } },
    ],
  ])("a fact with an invalid source: %s", (_label, factOverride) => {
    expect(validateModelSnapshot(withFact(factOverride))).toEqual({
      status: "refused",
      refusal: "policy.snapshot_unsourced",
    });
  });
});

describe("validateModelSnapshot — malformed (structural / identity)", () => {
  test.each([
    ["unknown top-level key", { extra: 1 }],
    ["wrong schemaVersion", { schemaVersion: "libre-ai.model-snapshot.v1" }],
    ["id not a snapshot urn", { id: "urn:libre-ai:policy:x" }],
    ["tenantId without ten_", { tenantId: "org-x" }],
    ["modelId without mdl_", { modelId: "model-x" }],
    ["capturedAt not UTC seconds", { capturedAt: "2026-07-21T10:00:00+02:00" }],
    ["digest not sha256", { digest: "z".repeat(64) }],
    ["facts empty", { facts: [] }],
  ])("is malformed: %s", (_label, override) => {
    expect(validateModelSnapshot(raw(override))).toEqual({ status: "malformed" });
  });

  test.each([
    ["fact name not model-scoped", { name: "need.tenure" }],
    ["fact name malformed", { name: "model.BAD" }],
    ["fact value out of scale", { value: 9007199254740992 }],
    ["fact unknown key", { name: "model.provenance", value: "x", source: SOURCE, note: "y" }],
  ])("is malformed via a bad fact: %s", (_label, factOverride) => {
    expect(validateModelSnapshot(withFact(factOverride))).toEqual({ status: "malformed" });
  });

  test("duplicate fact names", () => {
    expect(validateModelSnapshot(raw({ facts: [FACT, FACT] }))).toEqual({ status: "malformed" });
  });

  test("a non-object input is malformed", () => {
    expect(validateModelSnapshot(null)).toEqual({ status: "malformed" });
    expect(validateModelSnapshot([])).toEqual({ status: "malformed" });
  });
});
