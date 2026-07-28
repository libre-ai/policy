import { expect, test } from "bun:test";
import { resolve } from "node:path";

test("Model Policy phase checker keeps its blocking coverage floor", async () => {
  const childProcess = Bun.spawn([process.execPath, "run", "test:product-phases:coverage"], {
    cwd: resolve(import.meta.dir, ".."),
    stdout: "pipe",
    stderr: "pipe",
  });
  const [stdout, stderr, exitCode] = await Promise.all([
    new Response(childProcess.stdout).text(),
    new Response(childProcess.stderr).text(),
    childProcess.exited,
  ]);
  expect(exitCode, `Coverage gate failed.\nstdout:\n${stdout}\nstderr:\n${stderr}`).toBe(0);
}, 30_000);
