// RFC 8785 JSON Canonicalization, RFC 2119 keywords, SEMANTICS.md §3-7
export type RawDefect =
  | "bom"
  | "invalid-utf8"
  | "duplicate-member"
  | "unpaired-surrogate"
  | "max-depth"
  | "invalid-number"
  | "invalid-json";

export type ErrorCode =
  | "input-invalid"
  | "evaluated-at-invalid"
  | "rule-id-duplicate"
  | "approval-invalid"
  | "digest-mismatch"
  | "tenant-mismatch";

export type RuleStatus = "satisfied" | "failed" | "unknown";
export type ReasonCode =
  | "policy.rule_satisfied"
  | "policy.rule_failed"
  | "policy.source_from_future"
  | "policy.snapshot_stale"
  | "policy.fact_type_mismatch"
  | "policy.fact_absent";

export type Verdict = "eligible" | "ineligible" | "indeterminate";

export type Operator = "equals" | "not-equals" | "in" | "not-in" | "at-least" | "at-most";

export type RuleValue = string | number | boolean | (string | number | boolean)[];

export type FactValue = string | number | boolean;

export interface JsonRecord {
  [key: string]: unknown;
}

export interface UnknownDisposition {
  unknown: "ineligible" | "indeterminate";
}

export interface PolicyRule extends UnknownDisposition {
  id: string;
  fact: string;
  operator: Operator;
  value: RuleValue;
  source: {
    uri: string;
    retrievedAt: string;
    digest: string;
    licence: string;
  };
  maxSourceAgeDays?: number;
}

export interface PolicyDefinition {
  schemaVersion: string;
  id: string;
  tenantId: string;
  version: number;
  status: string;
  digest: string;
  proposedBy: string;
  rules: PolicyRule[];
  approval: {
    actorKind: string;
    approverId: string;
    subjectDigest: string;
  };
}

export interface FactSource {
  retrievedAt: string;
  digest: string;
  licence: string;
}

export interface FactObject {
  name: string;
  value: FactValue;
  source: FactSource;
}

export interface ModelSnapshot {
  schemaVersion: string;
  id: string;
  tenantId: string;
  modelId: string;
  capturedAt: string;
  digest: string;
  facts: FactObject[];
}

export interface PolicyNeedFact {
  name: string;
  value: FactValue;
}

export interface PolicyNeed {
  schemaVersion: string;
  id: string;
  tenantId: string;
  digest: string;
  facts: PolicyNeedFact[];
}

export interface RuleResult {
  ruleId: string;
  status: RuleStatus;
  reasonCode: ReasonCode;
}

export interface PolicyEvaluation {
  schemaVersion: string;
  id: string;
  tenantId: string;
  policyId: string;
  policyDigest: string;
  snapshotId: string;
  snapshotDigest: string;
  needDigest: string;
  engineVersion: string;
  verdict: Verdict;
  ruleResults: RuleResult[];
  evaluatedAt: string;
  digest: string;
}

export class StrictJsonError extends Error {
  constructor(
    readonly defect: RawDefect,
    message: string,
  ) {
    super(message);
    this.name = "StrictJsonError";
  }
}
