wit_bindgen::generate!({
    world: "policy-core",
    path: "../../contracts/wit/policy-core-v1",
});

use self::exports::libre_ai::policy_core::api as wit_api;

use crate::{evaluate as core_evaluate, types::ErrorCode};

struct PolicyCoreComponent;

impl wit_api::Guest for PolicyCoreComponent {
    fn evaluate(
        policy: Vec<u8>,
        snapshot: Vec<u8>,
        need: Vec<u8>,
        evaluated_at: String,
    ) -> Result<Vec<u8>, wit_api::ContractError> {
        core_evaluate(&policy, &snapshot, &need, &evaluated_at).map_err(|err| {
            wit_api::ContractError {
                code: error_code_to_string(err),
                message: error_message(err),
            }
        })
    }
}

fn error_code_to_string(code: ErrorCode) -> String {
    code.as_str().to_string()
}

fn error_message(code: ErrorCode) -> String {
    match code {
        ErrorCode::InputInvalid => "Input JSON is invalid or malformed".to_string(),
        ErrorCode::EvaluatedAtInvalid => {
            "evaluated-at timestamp format or value is invalid".to_string()
        }
        ErrorCode::RuleIdDuplicate => "Duplicate rule ID found in policy".to_string(),
        ErrorCode::ApprovalInvalid => "Policy approval is invalid or missing".to_string(),
        ErrorCode::DigestMismatch => {
            "Policy, snapshot, or need digest does not match content".to_string()
        }
        ErrorCode::TenantMismatch => "Tenant ID mismatch between policy and snapshot".to_string(),
    }
}

export!(PolicyCoreComponent);
