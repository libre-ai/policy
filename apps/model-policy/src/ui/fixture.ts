// A small, deterministic set of approved policy definitions used to render the
// read-only cockpit in tests and local development. Per the spec's runtime
// boundaries the cockpit uses contract fixtures; it authors nothing and evaluates
// nothing (the deterministic rule evaluator is the deferred Rust/WASM boundary).
// These fixtures are domain-VALID (they pass validatePolicyDefinition), so they
// honestly represent approved policies — a co-located test asserts it.

import type { PolicyDefinition, PolicySource, Rule } from "../domain/policy-definition";

const TENANT = "ten_aaaaaaaaaaaaaaaa";

function source(): PolicySource {
  return {
    uri: "https://example.org/policy",
    retrievedAt: "2030-01-01T00:00:00Z",
    digest: "a".repeat(64),
    licence: "CC-BY-4.0",
  };
}

function rule(id: string, fact: string, value: Rule["value"]): Rule {
  return {
    id,
    fact,
    operator: "equals",
    value,
    unknown: "ineligible",
    source: source(),
  };
}

export const COCKPIT_FIXTURE: readonly PolicyDefinition[] = [
  {
    schemaVersion: "libre-ai.policy-definition.v2",
    id: "urn:libre-ai:policy:0001",
    tenantId: TENANT,
    version: 1,
    status: "approved",
    proposedBy: "usr_aaaaaaaaaaaaaaaa",
    rules: [
      rule("rule-a", "model.provider", "anthropic"),
      rule("rule-b", "model.hosted-in-eu", true),
    ],
    digest: "b".repeat(64),
    approvedAt: "2030-01-02T00:00:00Z",
    approval: {
      role: "policy-approver",
      actorKind: "human",
      approverId: "usr_bbbbbbbbbbbbbbbb",
      approvedAt: "2030-01-02T00:00:00Z",
      reference: "urn:libre-ai:approval:0001",
      subjectDigest: "c".repeat(64),
    },
  },
  {
    schemaVersion: "libre-ai.policy-definition.v2",
    id: "urn:libre-ai:policy:0002",
    tenantId: TENANT,
    version: 3,
    status: "approved",
    proposedBy: "usr_cccccccccccccccc",
    rules: [
      rule("rule-a", "model.provider", "anthropic"),
      rule("rule-b", "model.hosted-in-eu", true),
      rule("rule-c", "model.family", "claude"),
    ],
    digest: "d".repeat(64),
    approvedAt: "2030-01-03T00:00:00Z",
    approval: {
      role: "policy-approver",
      actorKind: "human",
      approverId: "usr_dddddddddddddddd",
      approvedAt: "2030-01-03T00:00:00Z",
      reference: "urn:libre-ai:approval:0002",
      subjectDigest: "e".repeat(64),
    },
  },
  {
    schemaVersion: "libre-ai.policy-definition.v2",
    id: "urn:libre-ai:policy:0003",
    tenantId: TENANT,
    version: 2,
    status: "approved",
    proposedBy: "usr_eeeeeeeeeeeeeeee",
    rules: [rule("rule-a", "model.provider", "anthropic")],
    digest: "f".repeat(64),
    approvedAt: "2030-01-04T00:00:00Z",
    approval: {
      role: "policy-approver",
      actorKind: "human",
      approverId: "usr_ffffffffffffffff",
      approvedAt: "2030-01-04T00:00:00Z",
      reference: "urn:libre-ai:approval:0003",
      subjectDigest: "0".repeat(64),
    },
  },
];
