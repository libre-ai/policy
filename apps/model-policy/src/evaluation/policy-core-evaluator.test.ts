import { describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { resolve } from "node:path";

import { CONTRACT_ERROR_CODES, mapContractError } from "./error-mapping";
import { createPolicyCoreEvaluator } from "./policy-core-evaluator";

// mapContractError is pure — always exercised, no WASM toolchain required.
describe("mapContractError", () => {
  test("passes every contract code through with its message", () => {
    for (const code of CONTRACT_ERROR_CODES) {
      expect(mapContractError(code, "why")).toEqual({ code, message: "why" });
    }
  });

  test("fails closed to engine-unknown on an unlisted code", () => {
    expect(mapContractError("brand-new-code", "why")).toEqual({
      code: "engine-unknown",
      message: "why",
    });
    expect(mapContractError(undefined, "why").code).toBe("engine-unknown");
  });

  test("falls back to a stable message when none is provided", () => {
    expect(mapContractError("tenant-mismatch", undefined).message).toBe(
      "policy-core evaluation refused",
    );
    expect(mapContractError("tenant-mismatch", "").message).toBe("policy-core evaluation refused");
  });
});

// Live integration: needs the built artifact (node tools/quality/build-policy-core-wasm.ts).
// Skipped when absent so the toolchain-free bun-quality job stays green; the
// exhaustive 20/20 gate lives in tools/quality/policy-core-wasm-conformance.ts.
const generatedDirectory = resolve(
  import.meta.dir,
  "../../../../target/policy-core-wasm/generated",
);
const artifactBuilt = existsSync(resolve(generatedDirectory, "policy-core.js"));

const golden = artifactBuilt
  ? ((await Bun.file(
      resolve(import.meta.dir, "../../../../contracts/fixtures/policy-core-v2/golden.json"),
    ).json()) as {
      cases: Array<{
        id: string;
        policy: unknown;
        snapshot: unknown;
        need: unknown;
        evaluatedAt: string;
        expectedEvaluation?: Record<string, unknown>;
        expectedError?: { variant: string };
      }>;
    })
  : { cases: [] };

const encoder = new TextEncoder();
function bytes(value: unknown): Uint8Array {
  return encoder.encode(JSON.stringify(value));
}

describe.skipIf(!artifactBuilt)("policy-core WASM evaluator (live)", () => {
  test("evaluates a success vector to byte-exact policy-evaluation bytes", async () => {
    const evaluator = await createPolicyCoreEvaluator(generatedDirectory);
    const vector = golden.cases.find((c) => c.expectedEvaluation);
    const expectedEvaluation = vector?.expectedEvaluation;
    if (!vector || !expectedEvaluation) throw new Error("no success vector in golden.json");
    const result = evaluator.evaluate(
      bytes(vector.policy),
      bytes(vector.snapshot),
      bytes(vector.need),
      vector.evaluatedAt,
    );
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(JSON.parse(new TextDecoder().decode(result.value))).toEqual(expectedEvaluation);
  });

  test("maps a contract error vector to a typed refusal", async () => {
    const evaluator = await createPolicyCoreEvaluator(generatedDirectory);
    const vector = golden.cases.find((c) => c.expectedError);
    const expectedError = vector?.expectedError;
    if (!vector || !expectedError) throw new Error("no error vector in golden.json");
    const result = evaluator.evaluate(
      bytes(vector.policy),
      bytes(vector.snapshot),
      bytes(vector.need),
      vector.evaluatedAt,
    );
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.code as string).toBe(expectedError.variant);
  });
});
