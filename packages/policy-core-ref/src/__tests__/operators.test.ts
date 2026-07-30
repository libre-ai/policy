import { describe, expect, it } from "bun:test";
import { readFileSync } from "node:fs";
import { digest, type ErrorCode, evaluate, type FactValue } from "../index";
import { normalize } from "../normalize";

interface OperatorVector {
  id: string;
  operator: string;
  ruleValue: unknown;
  observedValues: unknown[];
  expected?: {
    status: string;
    reasonCode: string;
  };
  expectedError?: string;
}

interface OperatorsFile {
  schemaVersion: string;
  semantics: string;
  vectors: OperatorVector[];
  aggregationVectors: OperatorVector[];
  invalidPolicyVectors: Array<{
    id: string;
    operator: string;
    ruleValue: unknown;
    expectedError: string;
  }>;
}

// Load fixture data synchronously at file parse time
const operatorsContent = readFileSync("contracts/fixtures/policy-core-v2/operators.json", "utf8");
const operatorsData: OperatorsFile = JSON.parse(operatorsContent);
const operatorVectors = operatorsData.vectors;
const aggregationVectors = operatorsData.aggregationVectors;
const invalidPolicyVectors = operatorsData.invalidPolicyVectors;

/**
 * Test helper: construct a minimal policy/snapshot/need for operator testing
 */
function constructTestInputs(
  operator: string,
  ruleValue: unknown,
  observedValues: unknown[],
): {
  policy: unknown;
  snapshot: unknown;
  need: unknown;
} {
  // Build policy
  const rules = [
    {
      id: "operator_test_rule",
      fact: "model.test_value",
      operator,
      value: ruleValue,
      unknown: "indeterminate" as const,
      source: {
        uri: "https://example.org/test",
        retrievedAt: "2026-01-01T00:00:00Z",
        digest: "a".repeat(64),
        licence: "CC-BY-4.0",
      },
    },
  ];

  // Policy digest is computed from a specific subject (see evaluator.ts:70-82)
  const policySubject = {
    schemaVersion: "libre-ai.policy-definition.v2",
    id: "urn:libre-ai:policy:operator-test",
    tenantId: "ten_test",
    version: 1,
    status: "approved",
    proposedBy: "svc_test",
    rules,
  };

  // Normalize before computing digest (matches evaluator.ts:81)
  const normalizedPolicySubject = normalize({ ...policySubject }, "policy") as typeof policySubject;
  const policyDigest = digest("libre-ai.policy-definition.v2", normalizedPolicySubject);

  const policy = {
    ...policySubject,
    digest: policyDigest,
    approvedAt: "2026-01-02T00:00:00Z",
    approval: {
      role: "policy-approver",
      approvedAt: "2026-01-02T00:00:00Z",
      reference: "urn:libre-ai:approval:operator-test",
      subjectDigest: policyDigest,
      actorKind: "human" as const,
      approverId: "usr_test",
    },
  };

  // Build snapshot
  const facts = observedValues.map((value) => ({
    name: "model.test_value",
    value,
    source: {
      uri: "https://example.org/test-fact",
      retrievedAt: "2026-01-01T00:00:00Z",
      digest: "b".repeat(64),
      licence: "CC-BY-4.0",
    },
  }));

  const snapshotBase = {
    schemaVersion: "libre-ai.model-snapshot.v2",
    id: "urn:libre-ai:snapshot:operator-test",
    tenantId: "ten_test",
    modelId: "mdl_test",
    capturedAt: "2026-01-01T00:00:00Z",
    facts,
  };

  // Normalize before computing digest (matches evaluator.ts:85)
  const normalizedSnapshot = normalize({ ...snapshotBase }, "snapshot") as typeof snapshotBase;
  const snapshotDigest = digest("libre-ai.model-snapshot.v2", normalizedSnapshot);
  const snapshot = {
    ...normalizedSnapshot,
    digest: snapshotDigest,
  };

  // Build need
  const needBase = {
    schemaVersion: "libre-ai.policy-need.v2",
    id: "urn:libre-ai:need:operator-test",
    tenantId: "ten_test",
    facts: [
      {
        name: "need.task",
        value: "test",
      },
    ],
  };

  // Normalize before computing digest (matches evaluator.ts:89)
  const normalizedNeed = normalize({ ...needBase }, "need") as typeof needBase;
  const needDigest = digest("libre-ai.policy-need.v2", normalizedNeed);
  const need = {
    ...normalizedNeed,
    digest: needDigest,
  };

  return { policy, snapshot, need };
}

describe("operator predicate vectors (28 cases)", () => {
  for (let i = 0; i < operatorVectors.length; i++) {
    const vec = operatorVectors[i];
    if (!vec) {
      throw new Error(`Vector at index ${i} is undefined`);
    }

    it(`should satisfy operator vector ${vec.id}`, async () => {
      const { policy, snapshot, need } = constructTestInputs(
        vec.operator,
        vec.ruleValue,
        vec.observedValues,
      );

      const policyBytes = new TextEncoder().encode(JSON.stringify(policy));
      const snapshotBytes = new TextEncoder().encode(JSON.stringify(snapshot));
      const needBytes = new TextEncoder().encode(JSON.stringify(need));

      const result = await evaluate(
        new Uint8Array(policyBytes),
        new Uint8Array(snapshotBytes),
        new Uint8Array(needBytes),
        "2026-01-01T00:00:00Z",
      );

      if (!vec.expected) {
        throw new Error(`Operator vector ${vec.id} missing expected field`);
      }

      if (!result.ok) {
        throw new Error(`Operator vector evaluation failed: ${result.error}`);
      }
      const decoded = JSON.parse(new TextDecoder().decode(result.jcs));
      if (!decoded.ruleResults || !Array.isArray(decoded.ruleResults)) {
        throw new Error(`Invalid evaluation result: missing ruleResults`);
      }
      const ruleResult = decoded.ruleResults[0];
      if (!ruleResult) {
        throw new Error(`Expected at least one rule result`);
      }
      expect(ruleResult.status).toBe(vec.expected.status);
      expect(ruleResult.reasonCode).toBe(vec.expected.reasonCode);
    });
  }
});

describe("aggregation vectors (5 cases)", () => {
  for (let i = 0; i < aggregationVectors.length; i++) {
    const vec = aggregationVectors[i];
    if (!vec) {
      throw new Error(`Aggregation vector at index ${i} is undefined`);
    }

    it(`should satisfy aggregation vector ${vec.id}`, async () => {
      const { policy, snapshot, need } = constructTestInputs(
        vec.operator,
        vec.ruleValue,
        vec.observedValues,
      );

      const policyBytes = new TextEncoder().encode(JSON.stringify(policy));
      const snapshotBytes = new TextEncoder().encode(JSON.stringify(snapshot));
      const needBytes = new TextEncoder().encode(JSON.stringify(need));

      const result = await evaluate(
        new Uint8Array(policyBytes),
        new Uint8Array(snapshotBytes),
        new Uint8Array(needBytes),
        "2026-01-01T00:00:00Z",
      );

      if (!vec.expected) {
        throw new Error(`Aggregation vector ${vec.id} missing expected field`);
      }

      if (!result.ok) {
        throw new Error(`Operator vector evaluation failed: ${result.error}`);
      }
      const decoded = JSON.parse(new TextDecoder().decode(result.jcs));
      if (!decoded.ruleResults || !Array.isArray(decoded.ruleResults)) {
        throw new Error(`Invalid evaluation result: missing ruleResults`);
      }
      const ruleResult = decoded.ruleResults[0];
      if (!ruleResult) {
        throw new Error(`Expected at least one rule result`);
      }
      expect(ruleResult.status).toBe(vec.expected.status);
      expect(ruleResult.reasonCode).toBe(vec.expected.reasonCode);
    });
  }
});

describe("invalid policy vectors (10 cases)", () => {
  for (let i = 0; i < invalidPolicyVectors.length; i++) {
    const vec = invalidPolicyVectors[i];
    if (!vec) {
      throw new Error(`Invalid policy vector at index ${i} is undefined`);
    }

    it(`should reject invalid policy vector ${vec.id}`, async () => {
      // Build policy with invalid ruleValue
      const rules = [
        {
          id: "invalid_rule",
          fact: "model.test_value",
          operator: vec.operator,
          value: vec.ruleValue, // Invalid value (e.g., array for equals operator)
          unknown: "indeterminate" as const,
          source: {
            uri: "https://example.org/test",
            retrievedAt: "2026-01-01T00:00:00Z",
            digest: "a".repeat(64),
            licence: "CC-BY-4.0",
          },
        },
      ];

      const policySubject = {
        schemaVersion: "libre-ai.policy-definition.v2",
        id: "urn:libre-ai:policy:invalid-test",
        tenantId: "ten_test",
        version: 1,
        status: "approved",
        proposedBy: "svc_test",
        rules,
      };

      const normalizedPolicySubject = normalize(
        { ...policySubject },
        "policy",
      ) as typeof policySubject;
      const policyDigest = digest("libre-ai.policy-definition.v2", normalizedPolicySubject);

      const policy = {
        ...policySubject,
        digest: policyDigest,
        approvedAt: "2026-01-02T00:00:00Z",
        approval: {
          role: "policy-approver",
          approvedAt: "2026-01-02T00:00:00Z",
          reference: "urn:libre-ai:approval:invalid-test",
          subjectDigest: policyDigest,
          actorKind: "human" as const,
          approverId: "usr_test",
        },
      };

      const snapshotBase = {
        schemaVersion: "libre-ai.model-snapshot.v2",
        id: "urn:libre-ai:snapshot:invalid-test",
        tenantId: "ten_test",
        modelId: "mdl_test",
        capturedAt: "2026-01-01T00:00:00Z",
        facts: [
          {
            name: "model.test_value",
            value: "dummy",
            source: {
              uri: "https://example.org/test",
              retrievedAt: "2026-01-01T00:00:00Z",
              digest: "b".repeat(64),
              licence: "CC-BY-4.0",
            },
          },
        ],
      };

      const normalizedSnapshotBase = normalize(
        { ...snapshotBase },
        "snapshot",
      ) as typeof snapshotBase;
      const snapshotDigest = digest("libre-ai.model-snapshot.v2", normalizedSnapshotBase);
      const snapshot = {
        ...normalizedSnapshotBase,
        digest: snapshotDigest,
      };

      const needBase = {
        schemaVersion: "libre-ai.policy-need.v2",
        id: "urn:libre-ai:need:invalid-test",
        tenantId: "ten_test",
        facts: [
          {
            name: "need.task",
            value: "test",
          },
        ],
      };

      const normalizedNeedBase = normalize({ ...needBase }, "need") as typeof needBase;
      const needDigest = digest("libre-ai.policy-need.v2", normalizedNeedBase);
      const need = {
        ...normalizedNeedBase,
        digest: needDigest,
      };

      const policyBytes = new TextEncoder().encode(JSON.stringify(policy));
      const snapshotBytes = new TextEncoder().encode(JSON.stringify(snapshot));
      const needBytes = new TextEncoder().encode(JSON.stringify(need));

      const result = await evaluate(
        new Uint8Array(policyBytes),
        new Uint8Array(snapshotBytes),
        new Uint8Array(needBytes),
        "2026-01-01T00:00:00Z",
      );

      expect(result.ok).toBe(false);
      if (!result.ok) {
        // Map operators.json format (policy.input_invalid) to evaluator format (input-invalid)
        const expectedError = vec.expectedError
          .replace("policy.", "")
          .replace("_", "-") as ErrorCode;
        expect(result.error).toBe(expectedError);
      }
    });
  }
});

describe("unknown reason code priority (order-independence)", () => {
  /**
   * SEMANTICS.md §6: When multiple occurrences reduce to unknown with DIFFERENT
   * reason codes, the result MUST choose the highest-priority reason code, not
   * the first in input order. This test verifies deterministic, order-independent
   * output (SEMANTICS.md §3: "Input array order never affects a rule status").
   *
   * Test case: rule with equals operator, string rule value, and two facts:
   * - fact 1: value=123 (number), source.retrievedAt="2026-01-31" (1 day old, fresh)
   *   → type mismatch (occurrence type number != rule value type string)
   * - fact 2: value="ok" (string), source.retrievedAt="2026-12-31" (future)
   *   → source from future
   *
   * Expected reason: policy.source_from_future (higher priority than fact_type_mismatch).
   * Assertion: Both orderings (forward and reversed) produce identical output.
   */
  it("should select highest-priority reason code regardless of fact order", async () => {
    // Construct the base policy/snapshot/need structure
    const evaluatedAt = "2026-02-01T00:00:00Z";
    const rule = {
      id: "reason_priority_rule",
      fact: "model.test",
      operator: "equals",
      value: "expected_string", // string value
      maxSourceAgeDays: 1,
      unknown: "indeterminate" as const,
      source: {
        uri: "https://example.org/test",
        retrievedAt: "2026-01-01T00:00:00Z",
        digest: "a".repeat(64),
        licence: "CC-BY-4.0",
      },
    };

    const policySubject = {
      schemaVersion: "libre-ai.policy-definition.v2",
      id: "urn:libre-ai:policy:reason-priority-test",
      tenantId: "ten_test",
      version: 1,
      status: "approved",
      proposedBy: "svc_test",
      rules: [rule],
    };

    const normalizedPolicySubject = normalize(
      { ...policySubject },
      "policy",
    ) as typeof policySubject;
    const policyDigest = digest("libre-ai.policy-definition.v2", normalizedPolicySubject);

    const policy = {
      ...policySubject,
      digest: policyDigest,
      approvedAt: "2026-01-02T00:00:00Z",
      approval: {
        role: "policy-approver",
        approvedAt: "2026-01-02T00:00:00Z",
        reference: "urn:libre-ai:approval:reason-priority-test",
        subjectDigest: policyDigest,
        actorKind: "human" as const,
        approverId: "usr_test",
      },
    };

    // Two facts that will produce different unknown reason codes
    const factTypeMatchFirst = {
      name: "model.test",
      value: 123, // number → type mismatch vs string rule value
      source: {
        uri: "https://example.org/fact1",
        retrievedAt: "2026-01-31T00:00:00Z", // 1 day old, within maxSourceAgeDays
        digest: "b".repeat(64),
        licence: "CC-BY-4.0",
      },
    };

    const factSourceFromFuture = {
      name: "model.test",
      value: "ok", // string → matches type, but source is from future
      source: {
        uri: "https://example.org/fact2",
        retrievedAt: "2026-12-31T00:00:00Z", // far future
        digest: "c".repeat(64),
        licence: "CC-BY-4.0",
      },
    };

    // Helper to run evaluation with given fact order
    const runEvaluation = async (
      facts: Array<{
        name: string;
        value: FactValue;
        source: { uri: string; retrievedAt: string; digest: string; licence: string };
      }>,
    ) => {
      const snapshotBase = {
        schemaVersion: "libre-ai.model-snapshot.v2",
        id: "urn:libre-ai:snapshot:reason-priority-test",
        tenantId: "ten_test",
        modelId: "mdl_test",
        capturedAt: "2026-01-01T00:00:00Z",
        facts,
      };

      const normalizedSnapshot = normalize({ ...snapshotBase }, "snapshot") as typeof snapshotBase;
      const snapshotDigest = digest("libre-ai.model-snapshot.v2", normalizedSnapshot);
      const snapshot = {
        ...normalizedSnapshot,
        digest: snapshotDigest,
      };

      const needBase = {
        schemaVersion: "libre-ai.policy-need.v2",
        id: "urn:libre-ai:need:reason-priority-test",
        tenantId: "ten_test",
        facts: [
          {
            name: "need.task",
            value: "test",
          },
        ],
      };

      const normalizedNeed = normalize({ ...needBase }, "need") as typeof needBase;
      const needDigest = digest("libre-ai.policy-need.v2", normalizedNeed);
      const need = {
        ...normalizedNeed,
        digest: needDigest,
      };

      const policyBytes = new TextEncoder().encode(JSON.stringify(policy));
      const snapshotBytes = new TextEncoder().encode(JSON.stringify(snapshot));
      const needBytes = new TextEncoder().encode(JSON.stringify(need));

      return evaluate(
        new Uint8Array(policyBytes),
        new Uint8Array(snapshotBytes),
        new Uint8Array(needBytes),
        evaluatedAt,
      );
    };

    // Run with original order (type mismatch first)
    const resultForward = await runEvaluation([factTypeMatchFirst, factSourceFromFuture]);
    expect(resultForward.ok).toBe(true);
    if (!resultForward.ok) throw new Error("Forward evaluation failed");
    const decodedForward = JSON.parse(new TextDecoder().decode(resultForward.jcs));
    const ruleResultForward = decodedForward.ruleResults?.[0];
    expect(ruleResultForward).toBeDefined();
    expect(ruleResultForward.status).toBe("unknown");
    expect(ruleResultForward.reasonCode).toBe("policy.source_from_future");

    // Run with reversed order (source from future first)
    const resultReverse = await runEvaluation([factSourceFromFuture, factTypeMatchFirst]);
    expect(resultReverse.ok).toBe(true);
    if (!resultReverse.ok) throw new Error("Reverse evaluation failed");
    const decodedReverse = JSON.parse(new TextDecoder().decode(resultReverse.jcs));
    const ruleResultReverse = decodedReverse.ruleResults?.[0];
    expect(ruleResultReverse).toBeDefined();
    expect(ruleResultReverse.status).toBe("unknown");
    expect(ruleResultReverse.reasonCode).toBe("policy.source_from_future");

    // Both orderings MUST produce identical reason code (determinism requirement)
    expect(ruleResultForward.reasonCode).toBe(ruleResultReverse.reasonCode);
    // Both MUST produce the full identical JCS output (byte equality)
    expect(resultForward.jcs).toEqual(resultReverse.jcs);
  });
});
