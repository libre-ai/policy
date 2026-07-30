// The typed boundary between the policy-core WASM contract-error surface and the
// model-policy host. The WIT `api.evaluate` returns `result<list<u8>,
// contract-error>`; jco lifts the error arm into a thrown `ComponentError` whose
// `payload` is the `{ code, message }` record. This module turns that raw code
// into a closed union so an unlisted code from a future engine version is a typed
// boundary failure the host handles explicitly, never a silently-trusted string.

// The six codes crates/policy-core/src/types.rs ErrorCode::as_str emits. Kept in
// sync with that enum; a new Rust variant without a code here surfaces as
// `engine-unknown`.
export const CONTRACT_ERROR_CODES = [
  "input-invalid",
  "evaluated-at-invalid",
  "rule-id-duplicate",
  "approval-invalid",
  "digest-mismatch",
  "tenant-mismatch",
] as const;

export type ContractErrorCode = (typeof CONTRACT_ERROR_CODES)[number];

// `engine-unknown` is the fail-closed catch-all: a code the contract does not
// define is refused as an engine boundary failure, never assumed benign.
export type EvaluationErrorCode = ContractErrorCode | "engine-unknown";

export interface EvaluationError {
  code: EvaluationErrorCode;
  message: string;
}

export type EvaluationResult =
  | { ok: true; value: Uint8Array }
  | { ok: false; error: EvaluationError };

function isContractErrorCode(value: unknown): value is ContractErrorCode {
  return typeof value === "string" && (CONTRACT_ERROR_CODES as readonly string[]).includes(value);
}

// Lifts a raw policy-core contract-error code + message into a typed
// `EvaluationError`. An absent or non-string message becomes a stable fallback;
// an unrecognised code fails closed to `engine-unknown`.
export function mapContractError(code: unknown, message: unknown): EvaluationError {
  const text =
    typeof message === "string" && message.length > 0 ? message : "policy-core evaluation refused";
  return isContractErrorCode(code)
    ? { code, message: text }
    : { code: "engine-unknown", message: text };
}
