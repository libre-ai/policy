// Builds the capability-free policy-core WASM component and its jco glue, the
// runtime artifact the model-policy Bun BFF instantiates to evaluate policies.
//
// Pipeline (server-side host, no browser bundle):
//   cargo build wasm32-unknown-unknown --release   → core module
//   componentNew (wasm-tools)                       → component
//   componentWit                                    → assert the closed policy-core API surface
//   transpileBytes (jco, instantiation: "async")    → capability-free JS glue + core module
//
// This mirrors tools/qualification/notebook-core-v2/build.ts but leaner:
// policy-core is a pure deterministic evaluator making no crypto-qualification
// claim, so there is no fault build, trap injection, RSS benchmark or pinned-node
// device attestation — only the reproducible capability-free build the evaluator
// and the conformance gate consume.
//
// Run under node (jco's clean host; bun emits a non-fatal `tcp_wrap` worker
// warning at transpile time). Outputs to target/policy-core-wasm/generated/
// (gitignored) — the artifact is never committed.

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { dirname, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

import { transpileBytes } from "@bytecodealliance/jco-transpile";
import { componentNew, componentWit } from "@bytecodealliance/jco-transpile/wasm-tools";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = resolve(scriptDirectory, "../..");
const outputDirectory = resolve(repositoryRoot, "target/policy-core-wasm");
const generatedDirectory = resolve(outputDirectory, "generated");
const coreModulePath = resolve(
  repositoryRoot,
  "target/wasm32-unknown-unknown/release/policy_core.wasm",
);
const EXPECTED_EXPORT = "libre-ai:policy-core/api@1.0.0";
const TRANSPILER = "@bytecodealliance/jco-transpile@0.4.2";

// A capability-free, reproducible Rust build must not be steered by external
// compiler controls; the same guard notebook qualification applies.
const forbiddenRustBuildEnvironment = Object.keys(process.env).filter(
  (name) =>
    Boolean(process.env[name]) &&
    ([
      "CARGO_BUILD_RUSTC",
      "CARGO_BUILD_RUSTC_WRAPPER",
      "CARGO_BUILD_RUSTFLAGS",
      "CARGO_ENCODED_RUSTFLAGS",
      "CARGO_INCREMENTAL",
      "CARGO_TARGET_DIR",
      "RUSTC",
      "RUSTC_BOOTSTRAP",
      "RUSTC_WRAPPER",
      "RUSTC_WORKSPACE_WRAPPER",
      "RUSTFLAGS",
    ].includes(name) ||
      name.startsWith("CARGO_PROFILE_RELEASE_") ||
      name.startsWith("CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_")),
);
if (forbiddenRustBuildEnvironment.length > 0) {
  throw new Error(
    `policy-core WASM build forbids external Rust build controls: ${forbiddenRustBuildEnvironment.join(", ")}`,
  );
}

function run(command: string, arguments_: string[]): void {
  const result = spawnSync(command, arguments_, {
    cwd: repositoryRoot,
    encoding: "utf8",
    stdio: "pipe",
  });
  if (result.status !== 0) {
    process.stderr.write(result.stdout ?? "");
    process.stderr.write(result.stderr ?? "");
    throw new Error(`policy-core WASM build command failed: ${command}`);
  }
}

function safeOutputPath(name: string): string {
  const output = resolve(generatedDirectory, name);
  const relativePath = relative(generatedDirectory, output);
  if (relativePath.startsWith(`..${sep}`) || relativePath === "..") {
    throw new Error("transpiler emitted an unsafe path");
  }
  return output;
}

function sha256(value: Uint8Array): string {
  return createHash("sha256").update(value).digest("hex");
}

run("cargo", [
  "build",
  "--locked",
  "-p",
  "policy-core",
  "--release",
  "--target",
  "wasm32-unknown-unknown",
]);

const coreBytes = new Uint8Array(await readFile(coreModulePath));
const componentBytes = await componentNew(coreBytes);
const wit = await componentWit(componentBytes);
if (!wit.includes(`export ${EXPECTED_EXPORT};`) || wit.includes("import ")) {
  throw new Error("component WIT surface is not the closed policy-core API");
}

const transpiled = await transpileBytes(componentBytes, {
  emitTypescriptDeclarations: true,
  instantiation: "async",
  name: "policy-core",
  nodejsCompat: false,
  strict: true,
  wasiShim: false,
});
if (transpiled.imports.length !== 0) {
  throw new Error("transpiled component has imports");
}
const expectedExports = new Set(["api:instance", `${EXPECTED_EXPORT}:instance`]);
const actualExports = new Set(transpiled.exports.map(([name, kind]) => `${name}:${kind}`));
if (
  actualExports.size !== expectedExports.size ||
  [...expectedExports].some((value) => !actualExports.has(value))
) {
  throw new Error("transpiled component exports do not match the locked API");
}

await rm(outputDirectory, { force: true, recursive: true });
await mkdir(generatedDirectory, { recursive: true });
for (const [name, bytes] of Object.entries(transpiled.files)) {
  const output = safeOutputPath(name);
  await mkdir(dirname(output), { recursive: true });
  await writeFile(output, bytes);
}

// The consumer instantiates this core module with an empty import object; prove
// it imports nothing before it ships.
const transpiledCorePath = safeOutputPath("policy-core.core.wasm");
const transpiledCore = new WebAssembly.Module(await readFile(transpiledCorePath));
if (WebAssembly.Module.imports(transpiledCore).length !== 0) {
  throw new Error("transpiled core module has imports");
}

const generated = Object.fromEntries(
  await Promise.all(
    Object.keys(transpiled.files)
      .sort()
      .map(async (name) => {
        const bytes = await readFile(safeOutputPath(name));
        return [name, { bytes: bytes.length, sha256: sha256(bytes) }];
      }),
  ),
);
const manifest = {
  component: { bytes: componentBytes.length, sha256: sha256(componentBytes) },
  coreModule: { bytes: coreBytes.length, sha256: sha256(coreBytes) },
  generated,
  schemaVersion: "libre-ai.policy-core-wasm-build.v1",
  transpiler: TRANSPILER,
};
await writeFile(
  resolve(outputDirectory, "manifest.json"),
  `${JSON.stringify(manifest, null, 2)}\n`,
);

console.log(
  `policy-core WASM built: component=${manifest.component.sha256} core=${manifest.coreModule.sha256}`,
);
