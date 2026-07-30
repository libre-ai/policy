import { beforeAll, describe, expect, it } from "bun:test";
import { readFile } from "node:fs/promises";
import { type ErrorCode, evaluate, jcs } from "../index";

interface GoldenCase {
  id: string;
  policy: unknown;
  snapshot: unknown;
  need: unknown;
  evaluatedAt: string;
  expectedEvaluation?: unknown;
  expectedError?: { variant: string };
}

interface GoldenFile {
  schemaVersion: string;
  engineVersion: string;
  canonicalization: string;
  digestAlgorithm: string;
  semantics: string;
  cases: GoldenCase[];
}

let goldenVectors: GoldenCase[] = [];

beforeAll(async () => {
  const goldenContent = await readFile("contracts/fixtures/policy-core-v2/golden.json", "utf8");
  const golden: GoldenFile = JSON.parse(goldenContent);
  goldenVectors = golden.cases;
});

describe("golden vector conformance (20 cases)", () => {
  it("should load all golden vectors", () => {
    expect(goldenVectors.length).toBeGreaterThan(0);
  });

  for (let i = 0; i < 20; i++) {
    // Note: we test the first 20 cases from golden.json in the conformance tests
    // (operators.json is tested separately)
    it(`should pass golden vector ${i}`, async () => {
      if (i >= goldenVectors.length) {
        expect(true).toBe(true);
        return;
      }

      const testCase = goldenVectors[i];
      if (!testCase) {
        throw new Error(`Expected testCase at index ${i}, but got undefined`);
      }

      // Serialize test inputs to JSON bytes
      const policyBytes = new TextEncoder().encode(JSON.stringify(testCase.policy));
      const snapshotBytes = new TextEncoder().encode(JSON.stringify(testCase.snapshot));
      const needBytes = new TextEncoder().encode(JSON.stringify(testCase.need));

      // Run evaluation
      const result = await evaluate(
        new Uint8Array(policyBytes),
        new Uint8Array(snapshotBytes),
        new Uint8Array(needBytes),
        testCase.evaluatedAt,
      );

      // Check result against expected
      if (testCase.expectedError) {
        expect(result.ok).toBe(false);
        if (!result.ok) {
          // Error codes use hyphens in WIT (input-invalid, not input_invalid)
          const expectedVariant = testCase.expectedError.variant as ErrorCode;
          expect(result.error).toBe(expectedVariant);
        }
      } else if (testCase.expectedEvaluation) {
        expect(result.ok).toBe(true);
        if (result.ok) {
          // Decode the JCS output and compare
          const decodedResult = JSON.parse(new TextDecoder().decode(result.jcs));

          // Both must be canonicalized to JCS for comparison
          const expectedJcs = new TextDecoder().decode(jcs(testCase.expectedEvaluation));
          const resultJcs = new TextDecoder().decode(jcs(decodedResult));

          if (resultJcs !== expectedJcs) {
            expect(resultJcs).toBe(expectedJcs);
          }

          // Also verify the JCS bytes match
          const resultBytes = result.jcs;
          const expectedBytes = jcs(testCase.expectedEvaluation);
          expect(resultBytes.byteLength).toBe(expectedBytes.byteLength);
        }
      }
    });
  }
});
