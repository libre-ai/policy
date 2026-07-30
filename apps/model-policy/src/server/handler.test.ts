import { describe, expect, test } from "bun:test";
import { createModelPolicyHandler } from "./handler";

const handler = createModelPolicyHandler(() => "req_0000000000000000");

describe("model-policy cockpit handler", () => {
  test("serves the server-rendered cockpit at /", async () => {
    const response = await handler(new Request("https://model-policy.test/"));
    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toContain("text/html");
    const html = await response.text();
    expect(html).toContain("Politiques de modèle");
    expect(html).toContain("<caption>");
  });

  test("reports health as JSON", async () => {
    const response = await handler(new Request("https://model-policy.test/api/health"));
    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({
      service: "libre-ai-model-policy",
      status: "ok",
      version: "v1",
    });
  });

  test("an unknown route is not found", async () => {
    const response = await handler(new Request("https://model-policy.test/nope"));
    expect(response.status).toBe(404);
  });
});
