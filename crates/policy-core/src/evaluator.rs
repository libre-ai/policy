// Policy evaluation engine. SEMANTICS.md §2-8: validation, operators, freshness, verdict.

use crate::jcs;
use crate::normalize;
use crate::types::*;
use serde_json::{Value, json};

/// Result of evaluation: either successful JCS bytes or error code
pub type EvaluateResult = Result<Vec<u8>, ErrorCode>;

/// Evaluate a policy against a model snapshot and need
pub fn evaluate(
    policy_bytes: &[u8],
    snapshot_bytes: &[u8],
    need_bytes: &[u8],
    evaluated_at: &str,
) -> EvaluateResult {
    // SEMANTICS.md §2: byte length preflight
    if policy_bytes.len() > INPUT_LIMIT_POLICY
        || snapshot_bytes.len() > INPUT_LIMIT_SNAPSHOT
        || need_bytes.len() > INPUT_LIMIT_NEED
        || evaluated_at.len() > INPUT_LIMIT_EVALUATED_AT
    {
        return Err(ErrorCode::InputInvalid);
    }

    // Decode with strict JSON parser
    let policy: Value =
        serde_json::from_slice(policy_bytes).map_err(|_| ErrorCode::InputInvalid)?;
    let snapshot: Value =
        serde_json::from_slice(snapshot_bytes).map_err(|_| ErrorCode::InputInvalid)?;
    let need: Value = serde_json::from_slice(need_bytes).map_err(|_| ErrorCode::InputInvalid)?;

    // Validate inputs in order (SEMANTICS.md §2)
    validate_inputs(&policy, &snapshot, &need, evaluated_at)?;

    // Compute digests for comparison (SEMANTICS.md §9)
    let policy_subject = json!({
        "schemaVersion": policy.get("schemaVersion"),
        "id": policy.get("id"),
        "tenantId": policy.get("tenantId"),
        "version": policy.get("version"),
        "status": policy.get("status"),
        "proposedBy": policy.get("proposedBy"),
        "rules": policy.get("rules"),
    });
    let policy_digest = jcs::digest(
        "libre-ai.policy-definition.v2",
        &normalize::normalize(&policy_subject, "policy"),
    );
    let snapshot_digest = jcs::digest(
        "libre-ai.model-snapshot.v2",
        &normalize::normalize(&remove_field(&snapshot, "digest"), "snapshot"),
    );
    let need_digest = jcs::digest(
        "libre-ai.policy-need.v2",
        &normalize::normalize(&remove_field(&need, "digest"), "need"),
    );

    // SEMANTICS.md §2.6: verify digests
    if policy.get("digest").and_then(|v| v.as_str()).unwrap_or("") != policy_digest {
        return Err(ErrorCode::DigestMismatch);
    }
    if policy
        .get("approval")
        .and_then(|a| a.get("subjectDigest"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        != policy_digest
    {
        return Err(ErrorCode::DigestMismatch);
    }
    if snapshot
        .get("digest")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        != snapshot_digest
    {
        return Err(ErrorCode::DigestMismatch);
    }
    if need.get("digest").and_then(|v| v.as_str()).unwrap_or("") != need_digest {
        return Err(ErrorCode::DigestMismatch);
    }

    // Evaluate all rules
    let rule_results = evaluate_rules(&policy, &snapshot, &need, evaluated_at)?;

    // Compute verdict (SEMANTICS.md §7)
    let verdict = compute_verdict(&policy, &rule_results);

    // Build evaluation (SEMANTICS.md §9)
    let unsigned_evaluation = json!({
        "schemaVersion": "libre-ai.policy-evaluation.v2",
        "tenantId": policy.get("tenantId"),
        "policyId": policy.get("id"),
        "policyDigest": policy_digest,
        "snapshotId": snapshot.get("id"),
        "snapshotDigest": snapshot_digest,
        "needDigest": need_digest,
        "engineVersion": ENGINE_VERSION,
        "verdict": verdict,
        "ruleResults": rule_results,
        "evaluatedAt": evaluated_at,
    });

    let evaluation_digest = jcs::digest("libre-ai.policy-evaluation.v2", &unsigned_evaluation);

    // Build final evaluation with id and digest
    let final_evaluation = json!({
        "schemaVersion": "libre-ai.policy-evaluation.v2",
        "tenantId": policy.get("tenantId"),
        "policyId": policy.get("id"),
        "policyDigest": policy_digest,
        "snapshotId": snapshot.get("id"),
        "snapshotDigest": snapshot_digest,
        "needDigest": need_digest,
        "engineVersion": ENGINE_VERSION,
        "verdict": verdict,
        "ruleResults": rule_results,
        "evaluatedAt": evaluated_at,
        "id": format!("urn:libre-ai:evaluation:{}", evaluation_digest),
        "digest": evaluation_digest,
    });

    // Serialize to JCS
    let result_jcs = jcs::jcs(&final_evaluation);

    // SEMANTICS.md §2: output ceiling check
    if result_jcs.len() > OUTPUT_LIMIT_SUCCESS {
        return Err(ErrorCode::InputInvalid);
    }

    Ok(result_jcs)
}

fn validate_inputs(
    policy: &Value,
    snapshot: &Value,
    need: &Value,
    evaluated_at: &str,
) -> Result<(), ErrorCode> {
    // SEMANTICS.md §2.3: validate evaluated-at format (YYYY-MM-DDTHH:mm:ssZ)
    if !is_utc_seconds(evaluated_at) {
        return Err(ErrorCode::EvaluatedAtInvalid);
    }

    // SEMANTICS.md §2.4: check for duplicate rule IDs
    let rules = policy
        .get("rules")
        .and_then(|r| r.as_array())
        .ok_or(ErrorCode::InputInvalid)?;
    let mut rule_ids = std::collections::HashSet::new();
    for rule in rules {
        let id = rule
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(ErrorCode::InputInvalid)?;
        if !rule_ids.insert(id) {
            return Err(ErrorCode::RuleIdDuplicate);
        }
    }

    // SEMANTICS.md §3: check for exact duplicate facts (by JCS canonicalization)
    if has_duplicate_facts(snapshot)? {
        return Err(ErrorCode::InputInvalid);
    }
    if has_duplicate_facts(need)? {
        return Err(ErrorCode::InputInvalid);
    }

    // SEMANTICS.md §2.5: approval separation (approverId != proposedBy, actorKind = human)
    let approval = policy.get("approval").ok_or(ErrorCode::InputInvalid)?;
    let approver_id = approval
        .get("approverId")
        .and_then(|v| v.as_str())
        .ok_or(ErrorCode::InputInvalid)?;
    let proposed_by = policy
        .get("proposedBy")
        .and_then(|v| v.as_str())
        .ok_or(ErrorCode::InputInvalid)?;
    if approver_id == proposed_by {
        return Err(ErrorCode::ApprovalInvalid);
    }
    let actor_kind = approval
        .get("actorKind")
        .and_then(|v| v.as_str())
        .ok_or(ErrorCode::InputInvalid)?;
    if actor_kind != "human" {
        return Err(ErrorCode::ApprovalInvalid);
    }

    // SEMANTICS.md §2.7: tenant mismatch
    let policy_tenant = policy
        .get("tenantId")
        .and_then(|v| v.as_str())
        .ok_or(ErrorCode::InputInvalid)?;
    let snapshot_tenant = snapshot
        .get("tenantId")
        .and_then(|v| v.as_str())
        .ok_or(ErrorCode::InputInvalid)?;
    let need_tenant = need
        .get("tenantId")
        .and_then(|v| v.as_str())
        .ok_or(ErrorCode::InputInvalid)?;
    if policy_tenant != snapshot_tenant || policy_tenant != need_tenant {
        return Err(ErrorCode::TenantMismatch);
    }

    Ok(())
}

fn has_duplicate_facts(container: &Value) -> Result<bool, ErrorCode> {
    let facts = container
        .get("facts")
        .and_then(|f| f.as_array())
        .ok_or(ErrorCode::InputInvalid)?;

    let mut seen = std::collections::HashSet::new();
    for fact in facts {
        let fact_jcs = jcs::jcs(fact);
        if !seen.insert(fact_jcs) {
            return Ok(true); // Duplicate found
        }
    }
    Ok(false)
}

fn evaluate_rules(
    policy: &Value,
    snapshot: &Value,
    need: &Value,
    evaluated_at: &str,
) -> Result<Value, ErrorCode> {
    let rules = policy
        .get("rules")
        .and_then(|r| r.as_array())
        .ok_or(ErrorCode::InputInvalid)?;

    let mut results: Vec<Value> = Vec::new();

    for rule in rules {
        let rule_id = rule
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(ErrorCode::InputInvalid)?;
        let fact_name = rule
            .get("fact")
            .and_then(|v| v.as_str())
            .ok_or(ErrorCode::InputInvalid)?;
        let operator_str = rule
            .get("operator")
            .and_then(|v| v.as_str())
            .ok_or(ErrorCode::InputInvalid)?;
        let rule_value = rule.get("value").ok_or(ErrorCode::InputInvalid)?;
        let max_source_age_days = rule.get("maxSourceAgeDays").and_then(|v| v.as_u64());

        // Find fact objects from the appropriate namespace
        let fact_objects = if fact_name.starts_with("need.") {
            find_fact_objects(need, fact_name)?
        } else if fact_name.starts_with("model.") {
            find_fact_objects(snapshot, fact_name)?
        } else {
            return Err(ErrorCode::InputInvalid);
        };

        // SEMANTICS.md §3: zero occurrences -> unknown/fact_absent
        if fact_objects.is_empty() {
            results.push(json!({
                "ruleId": rule_id,
                "status": "unknown",
                "reasonCode": "policy.fact_absent"
            }));
            continue;
        }

        // SEMANTICS.md §3: multiple occurrences -> evaluate all, reduce
        let mut occurrence_statuses = Vec::new();
        for fact_obj in fact_objects {
            let fact_value = fact_obj.get("value").ok_or(ErrorCode::InputInvalid)?;
            let status = evaluate_occurrence(
                fact_value,
                operator_str,
                rule_value,
                max_source_age_days,
                &fact_obj,
                evaluated_at,
            )?;
            occurrence_statuses.push(status);
        }

        // SEMANTICS.md §6: reduce by priority: failed > unknown > satisfied
        let rule_result = reduce_occurrences(&occurrence_statuses);
        results.push(json!({
            "ruleId": rule_id,
            "status": rule_result.0,
            "reasonCode": rule_result.1
        }));
    }

    // SEMANTICS.md §6: sort by rule ID (ascending raw UTF-8)
    results.sort_by(|a, b| {
        let a_id = a.get("ruleId").and_then(|v| v.as_str()).unwrap_or("");
        let b_id = b.get("ruleId").and_then(|v| v.as_str()).unwrap_or("");
        a_id.as_bytes().cmp(b_id.as_bytes())
    });

    Ok(Value::Array(results))
}

fn evaluate_occurrence(
    occurrence: &Value,
    operator: &str,
    value: &Value,
    max_source_age_days: Option<u64>,
    fact_obj: &Value,
    evaluated_at: &str,
) -> Result<(String, String), ErrorCode> {
    // SEMANTICS.md §5: freshness check (for model facts with source)
    if let Some(source) = fact_obj.get("source")
        && !source.is_null()
        && let Some((status, reason)) =
            check_freshness(evaluated_at, fact_obj, max_source_age_days)?
    {
        return Ok((status, reason));
    }

    // SEMANTICS.md §4: operator evaluation
    evaluate_operator(occurrence, operator, value)
}

fn check_freshness(
    evaluated_at: &str,
    fact_obj: &Value,
    max_source_age_days: Option<u64>,
) -> Result<Option<(String, String)>, ErrorCode> {
    let source = fact_obj.get("source").ok_or(ErrorCode::InputInvalid)?;
    let retrieved_at = source
        .get("retrievedAt")
        .and_then(|v| v.as_str())
        .ok_or(ErrorCode::InputInvalid)?;

    let evaluated_timestamp =
        parse_utc_timestamp(evaluated_at).ok_or(ErrorCode::EvaluatedAtInvalid)?;
    let retrieved_timestamp = parse_utc_timestamp(retrieved_at).ok_or(ErrorCode::InputInvalid)?;

    let age_seconds = evaluated_timestamp - retrieved_timestamp;

    // SEMANTICS.md §5: source from future
    if age_seconds < 0 {
        return Ok(Some((
            "unknown".to_string(),
            "policy.source_from_future".to_string(),
        )));
    }

    // SEMANTICS.md §5: snapshot stale
    if let Some(max_days) = max_source_age_days {
        let maximum_age_seconds = (max_days as i64) * 86400;
        if age_seconds > maximum_age_seconds {
            return Ok(Some((
                "unknown".to_string(),
                "policy.snapshot_stale".to_string(),
            )));
        }
    }

    Ok(None)
}

fn evaluate_operator(
    occurrence: &Value,
    operator: &str,
    value: &Value,
) -> Result<(String, String), ErrorCode> {
    match operator {
        "equals" => {
            if !same_type(occurrence, value) {
                return Ok((
                    "unknown".to_string(),
                    "policy.fact_type_mismatch".to_string(),
                ));
            }
            if occurrence == value {
                Ok(("satisfied".to_string(), "policy.rule_satisfied".to_string()))
            } else {
                Ok(("failed".to_string(), "policy.rule_failed".to_string()))
            }
        }
        "not-equals" => {
            if !same_type(occurrence, value) {
                return Ok((
                    "unknown".to_string(),
                    "policy.fact_type_mismatch".to_string(),
                ));
            }
            if occurrence != value {
                Ok(("satisfied".to_string(), "policy.rule_satisfied".to_string()))
            } else {
                Ok(("failed".to_string(), "policy.rule_failed".to_string()))
            }
        }
        "in" => {
            if let Some(set) = value.as_array() {
                for item in set {
                    if value_equals(occurrence, item) {
                        return Ok(("satisfied".to_string(), "policy.rule_satisfied".to_string()));
                    }
                }
                Ok(("failed".to_string(), "policy.rule_failed".to_string()))
            } else {
                Ok(("failed".to_string(), "policy.rule_failed".to_string()))
            }
        }
        "not-in" => {
            if let Some(set) = value.as_array() {
                for item in set {
                    if value_equals(occurrence, item) {
                        return Ok(("failed".to_string(), "policy.rule_failed".to_string()));
                    }
                }
                Ok(("satisfied".to_string(), "policy.rule_satisfied".to_string()))
            } else {
                Ok(("failed".to_string(), "policy.rule_failed".to_string()))
            }
        }
        "at-least" => match (occurrence.as_f64(), value.as_f64()) {
            (Some(occ_num), Some(val_num)) => {
                if occ_num >= val_num {
                    Ok(("satisfied".to_string(), "policy.rule_satisfied".to_string()))
                } else {
                    Ok(("failed".to_string(), "policy.rule_failed".to_string()))
                }
            }
            _ => Ok((
                "unknown".to_string(),
                "policy.fact_type_mismatch".to_string(),
            )),
        },
        "at-most" => match (occurrence.as_f64(), value.as_f64()) {
            (Some(occ_num), Some(val_num)) => {
                if occ_num <= val_num {
                    Ok(("satisfied".to_string(), "policy.rule_satisfied".to_string()))
                } else {
                    Ok(("failed".to_string(), "policy.rule_failed".to_string()))
                }
            }
            _ => Ok((
                "unknown".to_string(),
                "policy.fact_type_mismatch".to_string(),
            )),
        },
        _ => Ok(("failed".to_string(), "policy.rule_failed".to_string())),
    }
}

fn value_equals(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => {
            // Compare as f64 for safe integer range
            if let (Some(xf), Some(yf)) = (x.as_f64(), y.as_f64()) {
                xf == yf
            } else {
                false
            }
        }
        (Value::String(x), Value::String(y)) => x == y,
        _ => false,
    }
}

fn reduce_occurrences(statuses: &[(String, String)]) -> (String, String) {
    // SEMANTICS.md §6: failed > unknown > satisfied
    for (status, reason) in statuses {
        if status == "failed" {
            return (status.clone(), reason.clone());
        }
    }

    // SEMANTICS.md §6: among unknowns, choose by fixed priority order
    let reason_priority = [
        "policy.source_from_future",
        "policy.snapshot_stale",
        "policy.fact_type_mismatch",
        "policy.fact_absent",
    ];

    for reason in reason_priority.iter() {
        for (status, s_reason) in statuses {
            if status == "unknown" && s_reason == reason {
                return (status.clone(), s_reason.clone());
            }
        }
    }

    ("satisfied".to_string(), "policy.rule_satisfied".to_string())
}

fn compute_verdict(policy: &Value, rule_results: &Value) -> String {
    let empty_vec = Vec::new();
    let results = rule_results.as_array().unwrap_or(&empty_vec);

    // Check for failed results
    for result in results {
        if result.get("status").and_then(|v| v.as_str()) == Some("failed") {
            return "ineligible".to_string();
        }
    }

    // Check for unknown results
    let mut has_unknown = false;
    for result in results {
        if result.get("status").and_then(|v| v.as_str()) == Some("unknown") {
            has_unknown = true;
            // Find corresponding rule to check unknown disposition
            let rule_id = result.get("ruleId").and_then(|v| v.as_str());
            if let Some(rid) = rule_id
                && let Some(rules) = policy.get("rules").and_then(|r| r.as_array())
            {
                for rule in rules {
                    if rule.get("id").and_then(|v| v.as_str()) == Some(rid)
                        && rule.get("unknown").and_then(|v| v.as_str()) == Some("ineligible")
                    {
                        return "ineligible".to_string();
                    }
                }
            }
        }
    }

    if has_unknown {
        return "indeterminate".to_string();
    }

    "eligible".to_string()
}

fn find_fact_objects(container: &Value, fact_name: &str) -> Result<Vec<Value>, ErrorCode> {
    let facts = container
        .get("facts")
        .and_then(|f| f.as_array())
        .ok_or(ErrorCode::InputInvalid)?;

    // Look for facts with the exact fact_name (e.g., "model.score" or "need.training_level")
    let matching: Vec<Value> = facts
        .iter()
        .filter(|f| f.get("name").and_then(|v| v.as_str()) == Some(fact_name))
        .cloned()
        .collect();

    Ok(matching)
}

fn is_utc_seconds(s: &str) -> bool {
    // YYYY-MM-DDTHH:mm:ssZ format, exactly 20 bytes
    if s.len() != 20 {
        return false;
    }
    parse_utc_timestamp(s).is_some()
}

fn parse_utc_timestamp(s: &str) -> Option<i64> {
    // Parse YYYY-MM-DDTHH:mm:ssZ to unix timestamp
    if s.len() != 20 || !s.ends_with('Z') {
        return None;
    }
    let datetime_str = &s[..19];
    let parts: Vec<&str> = datetime_str.split(&['T', '-', ':'][..]).collect();
    if parts.len() != 6 {
        return None;
    }

    let year: i32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;
    let hour: u32 = parts[3].parse().ok()?;
    let minute: u32 = parts[4].parse().ok()?;
    let second: u32 = parts[5].parse().ok()?;

    // Validation
    if !((1..=12).contains(&month)
        && (1..=31).contains(&day)
        && (0..=23).contains(&hour)
        && (0..=59).contains(&minute)
        && (0..=59).contains(&second))
    {
        return None;
    }

    // Validate day of month
    let days_in_month_table = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let max_day = if month == 2 && is_leap_year(year) {
        29
    } else {
        days_in_month_table[(month - 1) as usize]
    };

    if day > max_day as u32 {
        return None;
    }

    // Compute days since 1970-01-01
    let mut days: i64 = 0;
    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }

    let days_in_month = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let feb_days = if is_leap_year(year) { 29 } else { 28 };

    for m in 1..month {
        days += if m == 2 {
            feb_days as i64
        } else {
            days_in_month[(m - 1) as usize] as i64
        };
    }

    days += (day - 1) as i64;
    let seconds = days * 86400 + (hour as i64) * 3600 + (minute as i64) * 60 + (second as i64);

    Some(seconds)
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn remove_field(value: &Value, field: &str) -> Value {
    if let Some(obj) = value.as_object() {
        let mut new_obj = obj.clone();
        new_obj.remove(field);
        Value::Object(new_obj)
    } else {
        value.clone()
    }
}

fn same_type(a: &Value, b: &Value) -> bool {
    matches!(
        (a, b),
        (Value::Bool(_), Value::Bool(_))
            | (Value::Number(_), Value::Number(_))
            | (Value::String(_), Value::String(_))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_utc_seconds_valid() {
        assert!(is_utc_seconds("2026-01-01T00:00:00Z"));
    }

    #[test]
    fn test_is_utc_seconds_invalid_length() {
        assert!(!is_utc_seconds("2026-01-01T00:00:00"));
        assert!(!is_utc_seconds("2026-01-01T00:00:00Zabc"));
    }

    #[test]
    fn test_parse_utc_timestamp() {
        let ts = parse_utc_timestamp("2026-01-01T00:00:00Z");
        assert!(ts.is_some());
    }
}
