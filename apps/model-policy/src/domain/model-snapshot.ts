// Model-policy domain — the pure validator for a model snapshot
// (docs/apps/model-policy.md; contracts/schemas/model-snapshot.v2.schema.json).
// A snapshot is the sourced, content-addressed set of facts about a model that a
// policy is evaluated against. This module imports nothing, persists nothing,
// transmits nothing, and does NOT evaluate. It validates untrusted input into a
// typed, contract-conformant ModelSnapshot and reports, fail-closed:
//   - `malformed` — not a well-formed snapshot (identity/structure fails the
//     schema); a boundary concern, not a domain refusal.
//   - `refused`   — well-formed but a fact lacks a valid source/provenance
//     (`policy.snapshot_unsourced`).
//   - `valid`     — a typed, conformant snapshot.
// Patterns reuse the LOCKED common.v1 / model-snapshot.v2 $defs verbatim.

import { type FactScalar, hasExactKeys, isObject, validScalar } from "./fact-primitives";

export type { FactScalar };

const SNAPSHOT_ID = /^urn:libre-ai:snapshot:[A-Za-z0-9._~-]+$/;
const TENANT_ID = /^ten_[a-z0-9]{16,64}$/;
const MODEL_ID = /^mdl_[a-z0-9]{16,64}$/;
const SHA256 = /^[a-f0-9]{64}$/;
const FACT_NAME = /^model\.[a-z][a-z0-9_.-]+$/;
const HTTPS_URI =
  /^https:\/\/(?:[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?\.)+[A-Za-z]{2,63}(?::[0-9]{1,5})?(?:\/[A-Za-z0-9._~/-]*)?$/;
const UTC_SECONDS = /^[0-9]{4}-[0-9]{2}-[0-9]{2}T(?:[01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]Z$/;

export interface FactSource {
  readonly uri: string;
  readonly retrievedAt: string;
  readonly digest: string;
  readonly licence: string;
}
export interface Fact {
  readonly name: string;
  readonly value: FactScalar;
  readonly source: FactSource;
}
export interface ModelSnapshot {
  readonly schemaVersion: "libre-ai.model-snapshot.v2";
  readonly id: string;
  readonly tenantId: string;
  readonly modelId: string;
  readonly capturedAt: string;
  readonly facts: readonly Fact[];
  readonly digest: string;
}

export type SnapshotRefusalCode = "policy.snapshot_unsourced";

export type SnapshotValidation =
  | { readonly status: "valid"; readonly value: ModelSnapshot }
  | { readonly status: "malformed" }
  | { readonly status: "refused"; readonly refusal: SnapshotRefusalCode };

const MALFORMED: SnapshotValidation = { status: "malformed" };

function validSource(value: unknown): FactSource | undefined {
  if (!isObject(value) || !hasExactKeys(value, ["uri", "retrievedAt", "digest", "licence"]))
    return undefined;
  if (typeof value.uri !== "string" || value.uri.length > 2048 || !HTTPS_URI.test(value.uri))
    return undefined;
  if (typeof value.retrievedAt !== "string" || !UTC_SECONDS.test(value.retrievedAt))
    return undefined;
  if (typeof value.digest !== "string" || !SHA256.test(value.digest)) return undefined;
  if (typeof value.licence !== "string" || value.licence.length < 1 || value.licence.length > 128) {
    return undefined;
  }
  return Object.freeze({
    uri: value.uri,
    retrievedAt: value.retrievedAt,
    digest: value.digest,
    licence: value.licence,
  });
}

// A fact whose name/value is structurally bad is malformed; a well-formed fact
// missing a valid source is unsourced (policy.snapshot_unsourced).
type FactOutcome =
  | { readonly kind: "ok"; readonly fact: Fact }
  | { readonly kind: "malformed" }
  | { readonly kind: "unsourced" };

function validFact(value: unknown): FactOutcome {
  if (!isObject(value) || !hasExactKeys(value, ["name", "value", "source"]))
    return { kind: "malformed" };
  if (typeof value.name !== "string" || value.name.length > 128 || !FACT_NAME.test(value.name)) {
    return { kind: "malformed" };
  }
  if (!validScalar(value.value)) return { kind: "malformed" };
  if (!("source" in value)) return { kind: "unsourced" };
  const source = validSource(value.source);
  if (source === undefined) return { kind: "unsourced" };
  return { kind: "ok", fact: Object.freeze({ name: value.name, value: value.value, source }) };
}

const KEYS = [
  "schemaVersion",
  "id",
  "tenantId",
  "modelId",
  "capturedAt",
  "facts",
  "digest",
] as const;

/**
 * Validate untrusted input as a model snapshot. Identity/structural failures
 * return `malformed`; a well-formed snapshot with a fact that lacks a valid
 * source returns `policy.snapshot_unsourced`. Content-digest verification and
 * evaluation are deferred (no engine here).
 */
export function validateModelSnapshot(input: unknown): SnapshotValidation {
  if (!isObject(input) || !hasExactKeys(input, KEYS)) return MALFORMED;
  for (const key of KEYS) {
    if (!(key in input)) return MALFORMED;
  }
  if (input.schemaVersion !== "libre-ai.model-snapshot.v2") return MALFORMED;
  if (typeof input.id !== "string" || !SNAPSHOT_ID.test(input.id)) return MALFORMED;
  if (typeof input.tenantId !== "string" || !TENANT_ID.test(input.tenantId)) return MALFORMED;
  if (typeof input.modelId !== "string" || !MODEL_ID.test(input.modelId)) return MALFORMED;
  if (typeof input.capturedAt !== "string" || !UTC_SECONDS.test(input.capturedAt)) return MALFORMED;
  if (typeof input.digest !== "string" || !SHA256.test(input.digest)) return MALFORMED;
  if (!Array.isArray(input.facts) || input.facts.length < 1 || input.facts.length > 1000)
    return MALFORMED;

  const facts: Fact[] = [];
  for (const raw of input.facts) {
    const outcome = validFact(raw);
    if (outcome.kind === "malformed") return MALFORMED;
    if (outcome.kind === "unsourced")
      return { status: "refused", refusal: "policy.snapshot_unsourced" };
    facts.push(outcome.fact);
  }
  // Fact names are the stable identity; a duplicate name is a non-unique,
  // ambiguous snapshot (stricter than the schema's object-level uniqueItems).
  if (new Set(facts.map((f) => f.name)).size !== facts.length) return MALFORMED;

  return {
    status: "valid",
    value: Object.freeze({
      schemaVersion: "libre-ai.model-snapshot.v2",
      id: input.id,
      tenantId: input.tenantId,
      modelId: input.modelId,
      capturedAt: input.capturedAt,
      facts: Object.freeze(facts),
      digest: input.digest,
    }),
  };
}
