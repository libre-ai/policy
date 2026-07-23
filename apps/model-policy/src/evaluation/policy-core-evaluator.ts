// The model-policy evaluation adapter: it instantiates the capability-free
// policy-core WASM component (crates/policy-core, WIT world `policy-core`) and
// exposes a bytes-in/bytes-out deterministic `evaluate` matching the WIT exactly.
//
// Host boundary (docs/apps/model-policy.md §Runtime boundaries): the authorizing
// Bun BFF passes canonical policy/snapshot/need bytes and the explicit evaluation
// time; the component receives no token and imports NO host capability. This
// adapter enforces that by instantiating with an empty import object and refusing
// any core module that requests an import — evaluation cannot reach a clock,
// randomness, the network, the filesystem or the environment.
//
// The generated glue (target/policy-core-wasm/generated, gitignored) is produced
// by tools/quality/build-policy-core-wasm.ts. This module loads it by path at
// runtime, so it is not a compile-time import of the (uncommitted) artifact.

import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

import { type EvaluationResult, mapContractError } from "./error-mapping";

// The generated interface `libre-ai:policy-core/api@1.0.0` (evaluate is
// synchronous; the error arm is thrown, not returned).
interface PolicyCoreApi {
  evaluate(
    policy: Uint8Array,
    snapshot: Uint8Array,
    need: Uint8Array,
    evaluatedAt: string,
  ): Uint8Array;
}

// The jco `instantiation: "async"` glue. Structural type so `tsc` checks our use
// without importing the gitignored generated module.
interface PolicyCoreGlue {
  instantiate(
    getCoreModule: (path: string) => Promise<WebAssembly.Module>,
    imports: Record<string, never>,
    instantiateCore: (module: WebAssembly.Module) => Promise<WebAssembly.Instance>,
  ): Promise<{ api: PolicyCoreApi }>;
}

// jco throws a `ComponentError` whose `payload` carries the WIT contract-error
// record. Anything without such a payload is not a policy verdict (e.g. a trap).
interface ComponentErrorLike {
  payload?: { code?: unknown; message?: unknown };
}

function hasContractPayload(value: unknown): value is Required<ComponentErrorLike> {
  return (
    typeof value === "object" &&
    value !== null &&
    "payload" in value &&
    typeof (value as ComponentErrorLike).payload === "object" &&
    (value as ComponentErrorLike).payload !== null
  );
}

// The build output, relative to this file: apps/model-policy/src/evaluation →
// repo root → target/policy-core-wasm/generated.
const DEFAULT_GENERATED_DIRECTORY = resolve(
  import.meta.dir,
  "../../../../target/policy-core-wasm/generated",
);

export interface PolicyCoreEvaluator {
  // Deterministic: identical (policy, snapshot, need, evaluatedAt) bytes yield
  // byte-exact result bytes (policy-evaluation.v2, JCS-canonical).
  evaluate(
    policy: Uint8Array,
    snapshot: Uint8Array,
    need: Uint8Array,
    evaluatedAt: string,
  ): EvaluationResult;
}

// Instantiates the component once and returns a reusable evaluator. Pass an
// explicit `generatedDirectory` (tests, the conformance gate) to load a specific
// build; the default resolves the standard build output.
export async function createPolicyCoreEvaluator(
  generatedDirectory: string = DEFAULT_GENERATED_DIRECTORY,
): Promise<PolicyCoreEvaluator> {
  const glue = (await import(resolve(generatedDirectory, "policy-core.js"))) as PolicyCoreGlue;
  const root = await glue.instantiate(
    async (path) => {
      const module = await WebAssembly.compile(await readFile(resolve(generatedDirectory, path)));
      if (WebAssembly.Module.imports(module).length !== 0) {
        throw new Error("policy-core core module imports a host capability");
      }
      return module;
    },
    {},
    async (module) => WebAssembly.instantiate(module, {}),
  );
  const { api } = root;

  return {
    evaluate(policy, snapshot, need, evaluatedAt): EvaluationResult {
      try {
        return { ok: true, value: api.evaluate(policy, snapshot, need, evaluatedAt) };
      } catch (thrown) {
        if (hasContractPayload(thrown)) {
          return {
            ok: false,
            error: mapContractError(thrown.payload.code, thrown.payload.message),
          };
        }
        throw thrown;
      }
    },
  };
}
