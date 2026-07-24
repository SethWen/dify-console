use base64::{prelude::BASE64_STANDARD, Engine};
use reqwest::header::{HeaderMap, HeaderValue, COOKIE};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub id: String,
    pub name: String,
    pub mode: String,
    pub tags: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub name: String,
    pub app_id: String,
    pub mode: String,
}

pub struct DifyClient {
    client: reqwest::Client,
    base_url: String,
    cookies: Option<String>,
    csrf_token: Option<String>,
}

impl DifyClient {
    /// Create a new Dify client without an active session
    pub fn new(base_url: &str) -> Self {
        let base_url = base_url.trim_end_matches('/').to_string();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            client,
            base_url,
            cookies: None,
            csrf_token: None,
        }
    }

    /// Create a Dify client initialized with an existing session
    pub fn with_session(base_url: &str, cookies: &str, csrf_token: &str) -> Self {
        let mut client = Self::new(base_url);
        client.cookies = Some(cookies.to_string());
        client.csrf_token = Some(csrf_token.to_string());
        client
    }

    /// Get current session data
    #[allow(dead_code)]
    pub fn get_session(&self) -> Option<(String, String)> {
        match (&self.cookies, &self.csrf_token) {
            (Some(c), Some(t)) => Some((c.clone(), t.clone())),
            _ => None,
        }
    }

    /// Send HTTP request with session headers automatically added
    async fn send_request(
        &self,
        method: reqwest::Method,
        path: &str,
        query: Option<&[(String, String)]>,
        body: Option<Value>,
    ) -> Result<reqwest::Response, String> {
        let url = format!("{}{}", self.base_url, path);
        let mut builder = self.client.request(method, &url);

        if let Some(q) = query {
            builder = builder.query(q);
        }

        if let Some(b) = body {
            builder = builder.json(&b);
        }

        let mut headers = HeaderMap::new();
        if let Some(ref cookies) = self.cookies {
            headers.insert(COOKIE, HeaderValue::from_str(cookies).map_err(|e| e.to_string())?);
        }
        if let Some(ref csrf) = self.csrf_token {
            headers.insert(
                reqwest::header::HeaderName::from_static("x-csrf-token"),
                HeaderValue::from_str(csrf).map_err(|e| e.to_string())?,
            );
        }
        builder = builder.headers(headers);

        let response = builder.send().await.map_err(|e| e.to_string())?;
        Ok(response)
    }

    /// Authenticate with Dify WebUI Console API
    pub async fn login(&mut self, email: &str, password: &str) -> Result<(String, String), String> {
        let login_url = format!("{}/console/api/login", self.base_url);
        let encoded_password = BASE64_STANDARD.encode(password.as_bytes());

        let payload = serde_json::json!({
            "email": email,
            "password": encoded_password,
        });

        println!("[*] Attempting to login to Dify Console at {}...", self.base_url);

        let response = self.client.post(&login_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Login request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("Login failed with status {}: {}", status, text));
        }

        // Parse Cookies and CSRF token from Set-Cookie headers
        let mut cookie_parts = Vec::new();
        let mut csrf_token = None;
        for cookie_header in response.headers().get_all(reqwest::header::SET_COOKIE) {
            if let Ok(cookie_str) = cookie_header.to_str() {
                if let Some(first_part) = cookie_str.split(';').next() {
                    cookie_parts.push(first_part.to_string());
                    
                    let parts: Vec<&str> = first_part.split('=').collect();
                    if parts.len() == 2 {
                        let key = parts[0].trim();
                        let val = parts[1].trim();
                        if key == "csrf_token" || key == "__Host-csrf_token" {
                            csrf_token = Some(val.to_string());
                        }
                    }
                }
            }
        }

        let cookies = cookie_parts.join("; ");
        let csrf = csrf_token.ok_or_else(|| {
            "Could not find 'csrf_token' or '__Host-csrf_token' in Set-Cookie headers".to_string()
        })?;

        self.cookies = Some(cookies.clone());
        self.csrf_token = Some(csrf.clone());

        println!("[+] Login successful.");
        Ok((cookies, csrf))
    }

    /// Check if current session cookie and token are valid
    pub async fn check_session(&self) -> bool {
        if self.cookies.is_none() || self.csrf_token.is_none() {
            return false;
        }
        
        let query = vec![
            ("page".to_string(), "1".to_string()),
            ("limit".to_string(), "1".to_string()),
        ];
        
        match self.send_request(reqwest::Method::GET, "/console/api/apps", Some(&query), None).await {
            Ok(res) => res.status().is_success(),
            Err(_) => false,
        }
    }

    /// Fetch tag list from Dify Console and return ID of matching tag name
    pub async fn get_tag_id(&self, tag_name: &str) -> Result<Option<String>, String> {
        let query = vec![("type".to_string(), "app".to_string())];
        let res = self.send_request(reqwest::Method::GET, "/console/api/tags", Some(&query), None).await?;
        
        if !res.status().is_success() {
            return Err(format!("Failed to fetch tags: HTTP {}", res.status()));
        }

        let val: Value = res.json().await.map_err(|e| format!("Failed to parse tags JSON: {}", e))?;
        
        let tags = if let Some(arr) = val.as_array() {
            arr
        } else if let Some(arr) = val.get("data").and_then(|d| d.as_array()) {
            arr
        } else {
            return Ok(None);
        };

        for tag in tags {
            if let Some(name) = tag.get("name").and_then(|n| n.as_str()) {
                if name == tag_name {
                    if let Some(id) = tag.get("id").and_then(|i| i.as_str()) {
                        return Ok(Some(id.to_string()));
                    }
                }
            }
        }

        Ok(None)
    }

    /// List applications and filter them by modes and tags
    pub async fn list_apps(&self, modes: &[String], tag_name: Option<&str>) -> Result<Vec<AppInfo>, String> {
        let mut all_apps = Vec::new();
        let mut page = 1;
        let limit = 50;

        let mut tag_id = None;
        if let Some(t_name) = tag_name {
            println!("[*] Searching for tag '{}'...", t_name);
            match self.get_tag_id(t_name).await {
                Ok(Some(id)) => {
                    println!("[+] Found tag '{}' with ID: {}", t_name, id);
                    tag_id = Some(id);
                }
                Ok(None) => {
                    println!("[!] Warning: Tag '{}' not found on server. Falling back to client-side name match.", t_name);
                }
                Err(e) => {
                    println!("[!] Warning: Failed to fetch tag ID: {}. Falling back to client-side name match.", e);
                }
            }
        }

        println!("[*] Fetching apps with modes: {}...", modes.join(", "));

        loop {
            let mut query = vec![
                ("page".to_string(), page.to_string()),
                ("limit".to_string(), limit.to_string()),
                ("query".to_string(), "".to_string()),
            ];
            if let Some(ref tid) = tag_id {
                query.push(("tag_ids[0]".to_string(), tid.clone()));
            }

            let mut res = self.send_request(reqwest::Method::GET, "/console/api/apps", Some(&query), None).await?;

            // Fallback for older servers that don't support server-side tag filtering
            if res.status() == reqwest::StatusCode::BAD_REQUEST && tag_id.is_some() {
                println!("[!] Server returned 400. Retrying without server-side tag filtering...");
                query.retain(|(k, _)| k != "tag_ids[0]");
                res = self.send_request(reqwest::Method::GET, "/console/api/apps", Some(&query), None).await?;
            }

            if !res.status().is_success() {
                return Err(format!("Failed to fetch apps on page {}: HTTP {}", page, res.status()));
            }

            let val: Value = res.json().await.map_err(|e| format!("Failed to parse apps page JSON: {}", e))?;
            let apps_list = val.get("data")
                .and_then(|d| d.as_array())
                .ok_or_else(|| "Missing 'data' field in apps response".to_string())?;

            if apps_list.is_empty() {
                break;
            }

            let page_apps: Vec<AppInfo> = serde_json::from_value(Value::Array(apps_list.clone()))
                .map_err(|e| format!("Failed to deserialize app list: {}", e))?;

            // Filter by modes
            let mut filtered: Vec<AppInfo> = page_apps.into_iter()
                .filter(|app| modes.contains(&app.mode))
                .collect();

            // Filter by tag if tag_name is specified
            if let Some(t_name) = tag_name {
                filtered.retain(|app| {
                    if let Some(ref tags_val) = app.tags {
                        if let Some(tags_arr) = tags_val.as_array() {
                            for t in tags_arr {
                                if let Some(name) = t.get("name").and_then(|n| n.as_str()) {
                                    if name == t_name {
                                        return true;
                                    }
                                } else if let Some(name) = t.as_str() {
                                    if name == t_name {
                                        return true;
                                    }
                                }
                                if let Some(id) = t.get("id").and_then(|i| i.as_str()) {
                                    if let Some(ref tid) = tag_id {
                                        if id == tid {
                                            return true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    false
                });
            }

            println!("    - Page {}: fetched {} apps, found {} matching criteria.", page, apps_list.len(), filtered.len());
            all_apps.extend(filtered);

            let has_more = val.get("has_more").and_then(|h| h.as_bool()).unwrap_or(false);
            if !has_more || apps_list.len() < limit {
                break;
            }

            page += 1;
        }

        println!("[+] Total matching apps found: {}", all_apps.len());
        Ok(all_apps)
    }

    /// Export DSL for a single app ID
    pub async fn export_dsl(&self, app_id: &str, include_secret: bool) -> Result<String, String> {
        let path = format!("/console/api/apps/{}/export", app_id);
        let query = vec![("include_secret".to_string(), if include_secret { "true".to_string() } else { "false".to_string() })];

        let res = self.send_request(reqwest::Method::GET, &path, Some(&query), None).await?;
        if !res.status().is_success() {
            return Err(format!("HTTP {}", res.status()));
        }

        let val: Value = res.json().await.map_err(|e| format!("Failed to parse export JSON: {}", e))?;
        let dsl_content = val.get("data")
            .and_then(|d| d.as_str())
            .ok_or_else(|| "Missing 'data' field in export response".to_string())?;

        Ok(dsl_content.to_string())
    }

    /// Import or update DSL for an app ID
    pub async fn import_dsl(&self, yaml_content: &str, target_app_id: Option<&str>) -> Result<ImportResult, String> {
        let mut payload = serde_json::json!({
            "mode": "yaml-content",
            "yaml_content": yaml_content,
        });

        if let Some(id) = target_app_id {
            if let Some(obj) = payload.as_object_mut() {
                obj.insert("app_id".to_string(), Value::String(id.to_string()));
            }
        }

        let res = self.send_request(reqwest::Method::POST, "/console/api/apps/imports", None, Some(payload)).await?;
        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {}", status, text));
        }

        let result: ImportResult = res.json().await.map_err(|e| format!("Failed to parse import response: {}", e))?;
        Ok(result)
    }
}
