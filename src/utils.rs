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
