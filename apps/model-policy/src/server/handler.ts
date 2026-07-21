import { createRequestHandler, renderSsrDocument } from "@libre-ai/web-platform";
import { modelPolicyCockpitDocument } from "../shared/document";
import { COCKPIT_FIXTURE } from "../ui/fixture";

// The Model Policy cockpit request handler. The read view is server-rendered from
// a contract fixture (the spec's runtime boundary: no real policy authoring,
// approval or evaluation — evaluation is the deferred Rust/WASM boundary). No
// client assets are served — the view works without JavaScript.
export function createModelPolicyHandler(
  requestId: (request: Request) => string = () => `req_${crypto.randomUUID().replaceAll("-", "")}`,
): (request: Request) => Promise<Response> {
  return createRequestHandler({
    requestId,
    routes: {
      "/": () => renderSsrDocument(modelPolicyCockpitDocument(COCKPIT_FIXTURE)),
      "/api/health": () =>
        Response.json({ service: "libre-ai-model-policy", status: "ok", version: "v1" }),
    },
  });
}
