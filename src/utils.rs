use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaceRule {
    pub path: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvConfig {
    pub apps: HashMap<String, String>,
    pub replace_rules: Option<Vec<ReplaceRule>>,
}

pub fn sanitize_filename(name: &str) -> String {
    // Retains Chinese characters, alphanumeric characters, underscores, hyphens, and spaces.
    let re = regex::Regex::new(r"[^\w\s\-\u{4e00}-\u{9fa5}]").unwrap();
    let clean = re.replace_all(name, "");
    let re_spaces = regex::Regex::new(r"\s+").unwrap();
    let clean = re_spaces.replace_all(clean.trim(), "_");
    if clean.is_empty() {
        "unnamed_app".to_string()
    } else {
        clean.to_string()
    }
}

pub fn get_mode_folder(mode: &str) -> &str {
    match mode {
        "workflow" => "workflows",
        "advanced-chat" => "chatflows",
        _ => mode,
    }
}

pub fn parse_dsl_metadata(yaml: &str) -> (Option<String>, Option<String>) {
    let mut name = None;
    let mut mode = None;
    let mut in_app = false;

    for line in yaml.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Calculate indentation
        let indent = line.len() - line.trim_start().len();

        if indent == 0 {
            in_app = trimmed.starts_with("app:");
        } else if in_app && indent > 0 {
            if let Some(stripped) = trimmed.strip_prefix("name:") {
                let val = stripped.trim();
                let val = val.trim_matches(|c| c == '\'' || c == '"');
                name = Some(val.to_string());
            } else if let Some(stripped) = trimmed.strip_prefix("mode:") {
                let val = stripped.trim();
                let val = val.trim_matches(|c| c == '\'' || c == '"');
                mode = Some(val.to_string());
            }
        }
    }

    (name, mode)
}

fn apply_replace_rule(
    value: &mut serde_json::Value,
    path_segments: &[&str],
    target_key: &str,
    from_val: &str,
    to_val: &str,
) {
    if path_segments.is_empty() {
        if let Some(obj) = value.as_object_mut()
            && let Some(current_val) = obj.get_mut(target_key)
        {
            let current_str = match current_val {
                serde_json::Value::String(s) => s.clone(),
                _ => current_val.to_string(),
            };

            if from_val == "*" || current_str == from_val {
                let new_val = serde_json::from_str(to_val)
                    .unwrap_or_else(|_| serde_json::Value::String(to_val.to_string()));
                obj.insert(target_key.to_string(), new_val);
            }
        }
        return;
    }

    let current_seg = path_segments[0];
    let next_segments = &path_segments[1..];

    match value {
        serde_json::Value::Object(obj) => {
            if let Some(next_val) = obj.get_mut(current_seg) {
                apply_replace_rule(next_val, next_segments, target_key, from_val, to_val);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                apply_replace_rule(item, path_segments, target_key, from_val, to_val);
            }
        }
        _ => {}
    }
}

pub fn replace_yaml_content(yaml_content: &str, rules: &[ReplaceRule]) -> Result<String, String> {
    if rules.is_empty() {
        return Ok(yaml_content.to_string());
    }

    let mut value: serde_json::Value = serde_yaml::from_str(yaml_content)
        .map_err(|e| format!("Failed to parse YAML to JSON value: {}", e))?;

    for rule in rules {
        let parts: Vec<&str> = rule.path.split(':').collect();
        if parts.len() != 2 {
            println!(
                "    [!] Warning: Invalid replace_rule path format: '{}'. Expected 'path:key'.",
                rule.path
            );
            continue;
        }

        let path_str = parts[0];
        let target_key = parts[1];
        let path_segments: Vec<&str> = if path_str.is_empty() {
            vec![]
        } else {
            path_str.split('.').collect()
        };

        apply_replace_rule(&mut value, &path_segments, target_key, &rule.from, &rule.to);
    }

    let new_yaml = serde_yaml::to_string(&value)
        .map_err(|e| format!("Failed to serialize JSON value back to YAML: {}", e))?;

    // Post-process to fix naked '=' sign causing PyYAML tag constructor errors
    let final_yaml = new_yaml.replace("comparison_operator: =", "comparison_operator: '='");

    Ok(final_yaml)
}
