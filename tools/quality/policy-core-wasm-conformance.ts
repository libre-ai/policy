// The exhaustive live conformance gate for the policy-core WASM câblage: it
// instantiates the built component through the model-policy evaluation adapter and
// replays every golden vector (contracts/fixtures/policy-core-v2/golden.json),
// asserting the WASM path is byte-for-byte faithful to the normative SEMANTICS.md
// the native Rust engine already satisfies. It guards the residual risk the
// native conformance cannot see: a discrepancy introduced by the wasm32
// compilation or the jco canonical-ABI marshalling.
//
// Prerequisite: node tools/quality/build-policy-core-wasm.ts (produces the glue).
// Run under bun. Exits non-zero on any mismatch.

import { resolve } from "node:path";

import { createPolicyCoreEvaluator } from "../../apps/model-policy/src/evaluation/policy-core-evaluator";

const repositoryRoot = resolve(import.meta.dir, "../..");
const generatedDirectory = resolve(repositoryRoot, "target/policy-core-wasm/generated");
const goldenPath = resolve(repositoryRoot, "contracts/fixtures/policy-core-v2/golden.json");

interface GoldenCase {
  id: string;
  policy: unknown;
  snapshot: unknown;
  need: unknown;
  evaluatedAt: string;
  expectedEvaluation?: Record<string, unknown>;
  expectedError?: { variant: string };
}

function canonical(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(canonical);
  if (value !== null && typeof value === "object") {
    return Object.fromEntries(
      Object.keys(value as Record<string, unknown>)
        .sort()
        .map((key) => [key, canonical((value as Record<string, unknown>)[key])]),
    );
  }
  return value;
}

function deepEqual(a: unknown, b: unknown): boolean {
  return JSON.stringify(canonical(a)) === JSON.stringify(canonical(b));
}

const golden = (await Bun.file(goldenPath).json()) as {
  cases: GoldenCase[];
};
const evaluator = await createPolicyCoreEvaluator(generatedDirectory);
const encoder = new TextEncoder();
const decoder = new TextDecoder();

let passed = 0;
const failures: string[] = [];

for (const testCase of golden.cases) {
  const result = evaluator.evaluate(
    encoder.encode(JSON.stringify(testCase.policy)),
    encoder.encode(JSON.stringify(testCase.snapshot)),
    encoder.encode(JSON.stringify(testCase.need)),
    testCase.evaluatedAt,
  );

  if (testCase.expectedError) {
    if (result.ok) {
      failures.push(
        `${testCase.id}: expected error ${testCase.expectedError.variant}, got success`,
      );
    } else if (result.error.code !== testCase.expectedError.variant) {
      failures.push(
        `${testCase.id}: expected error ${testCase.expectedError.variant}, got ${result.error.code}`,
      );
    } else {
      passed += 1;
    }
    continue;
  }

  if (testCase.expectedEvaluation) {
    if (!result.ok) {
      failures.push(`${testCase.id}: expected success, got error ${result.error.code}`);
      continue;
    }
    const got = JSON.parse(decoder.decode(result.value));
    if (deepEqual(got, testCase.expectedEvaluation)) {
      passed += 1;
    } else {
      failures.push(
        `${testCase.id}: evaluation mismatch\n  got=${JSON.stringify(canonical(got))}\n  expected=${JSON.stringify(canonical(testCase.expectedEvaluation))}`,
      );
    }
    continue;
  }

  failures.push(`${testCase.id}: golden case has neither expectedEvaluation nor expectedError`);
}

const total = golden.cases.length;
if (failures.length > 0) {
  process.stderr.write(
    `policy-core WASM conformance: ${passed}/${total} pass, ${failures.length} fail\n`,
  );
  for (const failure of failures) process.stderr.write(`  FAIL ${failure}\n`);
  process.exit(1);
}
console.log(`policy-core WASM conformance: ${passed}/${total} golden vectors byte-exact`);
