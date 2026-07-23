// SEMANTICS.md §9: Normalization of unordered arrays for digest computation.
// Normalizes: rules (by id), in/not-in sets (by JCS), facts (by name, type-rank, JCS value/source).

use crate::jcs::jcs;
use serde_json::Value;

pub fn normalize(value: &Value, kind: &str) -> Value {
    let mut normalized = value.clone();
    match kind {
        "policy" => normalize_policy(&mut normalized),
        "snapshot" => normalize_snapshot(&mut normalized),
        "need" => normalize_need(&mut normalized),
        _ => {}
    }
    normalized
}

fn normalize_policy(value: &mut Value) {
    if let Some(rules) = value.get_mut("rules").and_then(|r| r.as_array_mut()) {
        // Sort rules by ascending id (raw UTF-8 bytes)
        rules.sort_by(|a, b| {
            let a_id = a.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let b_id = b.get("id").and_then(|v| v.as_str()).unwrap_or("");
            a_id.as_bytes().cmp(b_id.as_bytes())
        });

        // Normalize in/not-in sets within rules
        for rule in rules.iter_mut() {
            if let Some(operator) = rule.get("operator").and_then(|v| v.as_str())
                && (operator == "in" || operator == "not-in")
                && let Some(set_value) = rule.get_mut("value").filter(|v| v.is_array())
                && let Value::Array(items) = set_value
            {
                // Sort set members by ascending JCS canonical form
                items.sort_by_key(jcs);
            }
        }
    }
}

fn normalize_snapshot(value: &mut Value) {
    if let Some(facts) = value.get_mut("facts").and_then(|f| f.as_array_mut()) {
        // Sort by (name, type-rank, JCS(value), JCS(source))
        facts.sort_by(|a, b| {
            let a_name = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let b_name = b.get("name").and_then(|v| v.as_str()).unwrap_or("");

            let name_cmp = a_name.as_bytes().cmp(b_name.as_bytes());
            if name_cmp != std::cmp::Ordering::Equal {
                return name_cmp;
            }

            let a_rank = type_rank(a.get("value"));
            let b_rank = type_rank(b.get("value"));
            if a_rank != b_rank {
                return a_rank.cmp(&b_rank);
            }

            let value_cmp = jcs(a.get("value").unwrap_or(&Value::Null))
                .cmp(&jcs(b.get("value").unwrap_or(&Value::Null)));
            if value_cmp != std::cmp::Ordering::Equal {
                return value_cmp;
            }

            jcs(a.get("source").unwrap_or(&Value::Null))
                .cmp(&jcs(b.get("source").unwrap_or(&Value::Null)))
        });
    }
}

fn normalize_need(value: &mut Value) {
    if let Some(facts) = value.get_mut("facts").and_then(|f| f.as_array_mut()) {
        // Sort by (name, type-rank, JCS(value))
        facts.sort_by(|a, b| {
            let a_name = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let b_name = b.get("name").and_then(|v| v.as_str()).unwrap_or("");

            let name_cmp = a_name.as_bytes().cmp(b_name.as_bytes());
            if name_cmp != std::cmp::Ordering::Equal {
                return name_cmp;
            }

            let a_rank = type_rank(a.get("value"));
            let b_rank = type_rank(b.get("value"));
            if a_rank != b_rank {
                return a_rank.cmp(&b_rank);
            }

            jcs(a.get("value").unwrap_or(&Value::Null))
                .cmp(&jcs(b.get("value").unwrap_or(&Value::Null)))
        });
    }
}

// Type rank for sorting (boolean=0, number=1, string=2)
fn type_rank(value: Option<&Value>) -> u8 {
    match value {
        Some(Value::Bool(_)) => 0,
        Some(Value::Number(_)) => 1,
        Some(Value::String(_)) => 2,
        _ => 3, // unknown types ranked last
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_normalize_policy_sorts_rules() {
        let mut policy = json!({
            "rules": [
                {"id": "z_rule"},
                {"id": "a_rule"},
                {"id": "m_rule"}
            ]
        });
        normalize_policy(&mut policy);
        let rules = policy["rules"].as_array().unwrap();
        assert_eq!(rules[0]["id"], "a_rule");
        assert_eq!(rules[1]["id"], "m_rule");
        assert_eq!(rules[2]["id"], "z_rule");
    }

    #[test]
    fn test_normalize_policy_sorts_sets() {
        let mut policy = json!({
            "rules": [
                {
                    "id": "test",
                    "operator": "in",
                    "value": [3, 1, 2]
                }
            ]
        });
        normalize_policy(&mut policy);
        let set = policy["rules"][0]["value"].as_array().unwrap();
        // Should be sorted by JCS representation
        assert_eq!(set[0], 1);
        assert_eq!(set[1], 2);
        assert_eq!(set[2], 3);
    }
}
