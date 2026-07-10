# clearance-api — HTTP API

Read-only by construction: no mutating route exists. Policy and snapshot only
change via files + redeploy, so there is no application-level authz in v1 —
network exposure (VPN, reverse proxy, mTLS) is the organisation's deliberate
choice. The server binds `127.0.0.1:8080` by default and **refuses to boot**
on any invalid policy or snapshot (fail-closed).

```sh
cargo run -p rumble-ai-clearance-api --bin clearance-api -- \
  --rulebook content/rulebook/rulebook.yaml \
  --policy examples/policy-no-us-cn-selfhost-ok.yaml \
  --snapshot data/snapshot.json \
  --addr 127.0.0.1:8080
```

Every response uses the `{ "data": …, "meta": … }` envelope; `meta` cites the
snapshot version (`snapshot_generated_at`) wherever a decision depends on it.

## Endpoints

### `GET /api/v1/dataset`

Snapshot manifest: generation timestamp and dated sources.

```json
{ "data": { "generated_at": "2026-07-10T12:00:00Z", "sources": [ … ] }, "meta": {} }
```

### `GET /api/v1/models?cursor=<id>&limit=<n>`

Full catalogue, cursor-paginated (sorted by model id; `limit` ≤ 200, default
50). `meta.next_cursor` is the last id of the page, or `null` on the final
page.

### `POST /api/v1/evaluations`

Body: a need profile — same strict taxonomy as need files; unknown values are
refused with `422`.

```json
{ "task": "code_generation", "purpose": "public_content", "sensitivity": "c0" }
```

Response: eligible models **only**, ranked (task bench dimensions, then
blended price, then id), plus refusal counts:

```json
{
  "data": {
    "eligible": [{ "model": "mistralai/mistral-large-3" }, …],
    "ineligible_count": 1,
    "indeterminate_count": 1
  },
  "meta": { "snapshot_generated_at": "2026-07-10T12:00:00Z" }
}
```

### `GET /api/v1/verdicts?model=<id>&task=…&purpose=…&sensitivity=…`

Rule-by-rule verdict for one model. The model id goes in a query parameter
because ids contain slashes (`meta/llama-4`). A model absent from the snapshot
is denied (`builtin.unknown-model`), never 404-ed into ambiguity:

```json
{
  "data": {
    "model": "acme/never-heard-of",
    "verdict": { "ineligible": { "violations": ["builtin.unknown-model"] } }
  },
  "meta": { "snapshot_generated_at": "2026-07-10T12:00:00Z" }
}
```

Verdict shapes: `{"eligible": {"viable_hostings": […]}}`,
`{"ineligible": {"violations": [rule ids]}}`,
`{"indeterminate": {"missing": [[rule id, data dimension], …]}}` —
indeterminate is fail-closed (treated as not eligible).

### `GET /api/v1/policy`

The effective policy (rulebook ⊕ org after traced deactivations), rule by
rule — what the security team actually enforced, auditable over HTTP.

## Errors

- `422` — malformed need (unknown task/purpose/sensitivity value).
- `400` — malformed JSON body or query.
- Startup errors are fatal by design; there is no degraded mode.
