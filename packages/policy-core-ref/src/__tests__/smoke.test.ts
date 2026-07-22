import { describe, expect, it } from "bun:test";
import { digest, evaluate, jcs } from "../index";

describe("smoke tests", () => {
  it("should export evaluate function", () => {
    expect(typeof evaluate).toBe("function");
  });

  it("should export jcs function", () => {
    expect(typeof jcs).toBe("function");
  });

  it("should export digest function", () => {
    expect(typeof digest).toBe("function");
  });

  it("should handle invalid policy (oversized)", async () => {
    const oversized = new Uint8Array(8 * 1024 * 1024 + 1);
    const result = await evaluate(
      oversized,
      new Uint8Array([123]),
      new Uint8Array([123]),
      "2026-01-01T00:00:00Z",
    );
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error).toBe("input-invalid");
    }
  });

  it("should handle invalid evaluatedAt", async () => {
    const validJson = new Uint8Array(new TextEncoder().encode("{}"));
    const result = await evaluate(validJson, validJson, validJson, "not-a-date");
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error).toBe("evaluated-at-invalid");
    }
  });

  it("should canonicalize JSON correctly", () => {
    const value = { z: 1, a: 2, m: { b: 3, a: 1 } };
    const canonical = jcs(value);
    const str = new TextDecoder().decode(canonical);
    expect(str).toBe('{"a":2,"m":{"a":1,"b":3},"z":1}');
  });

  it("should compute digest correctly", () => {
    const value = { key: "value" };
    const digestResult = digest("test-label", value);
    expect(digestResult).toMatch(/^[0-9a-f]{64}$/);
  });
});
