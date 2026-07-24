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
