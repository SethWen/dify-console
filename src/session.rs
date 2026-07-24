use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub url: String,
    pub email: String,
    pub cookies: String,
    pub csrf_token: String,
    pub cached_at: u64,
}

fn get_cache_file_path() -> Option<PathBuf> {
    dirs::config_dir().map(|mut path| {
        path.push("dify-console");
        path.push("sessions.json");
        path
    })
}

fn load_sessions_map() -> HashMap<String, Session> {
    let path = match get_cache_file_path() {
        Some(p) => p,
        None => return HashMap::new(),
    };

    if !path.exists() {
        return HashMap::new();
    }

    let mut file = match File::open(&path) {
        Ok(f) => f,
        Err(_) => return HashMap::new(),
    };

    let mut content = String::new();
    if file.read_to_string(&mut content).is_err() {
        return HashMap::new();
    }

    serde_json::from_str(&content).unwrap_or_else(|_| HashMap::new())
}

pub fn get_cached_session(url: &str, email: &str) -> Option<Session> {
    let url_clean = url.trim_end_matches('/');
    let key = format!("{}|{}", url_clean, email);
    let map = load_sessions_map();
    map.get(&key).cloned()
}

pub fn save_session(url: &str, email: &str, cookies: &str, csrf_token: &str) -> Result<(), String> {
    let url_clean = url.trim_end_matches('/');
    let key = format!("{}|{}", url_clean, email);

    let mut map = load_sessions_map();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let session = Session {
        url: url_clean.to_string(),
        email: email.to_string(),
        cookies: cookies.to_string(),
        csrf_token: csrf_token.to_string(),
        cached_at: now,
    };

    map.insert(key, session);

    let path = get_cache_file_path().ok_or("Failed to locate config directory")?;

    // Ensure directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create config dir: {}", e))?;
    }

    let json_content = serde_json::to_string_pretty(&map)
        .map_err(|e| format!("Failed to serialize session mapping: {}", e))?;

    let mut file =
        File::create(&path).map_err(|e| format!("Failed to create session cache file: {}", e))?;

    file.write_all(json_content.as_bytes())
        .map_err(|e| format!("Failed to write session cache: {}", e))?;

    Ok(())
}
