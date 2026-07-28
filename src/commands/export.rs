use crate::dify_api::DifyClient;
use crate::utils::{get_mode_folder, sanitize_filename};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[allow(clippy::too_many_arguments)]
pub async fn run(
    url: String,
    email: String,
    password: Option<String>,
    output: String,
    modes: String,
    tag: Option<String>,
    env: String,
    map_file: String,
) -> Result<(), String> {
    // Process modes
    let available_modes = vec![
        "workflow",
        "advanced-chat",
        "chat",
        "agent-chat",
        "completion",
    ];
    let modes_to_export: Vec<String> = if modes.to_lowercase() == "all" {
        available_modes.into_iter().map(String::from).collect()
    } else {
        let parts: Vec<String> = modes.split(',').map(|m| m.trim().to_string()).collect();
        let invalid: Vec<&String> = parts
            .iter()
            .filter(|m| !available_modes.contains(&m.as_str()))
            .collect();
        if !invalid.is_empty() {
            let invalid_str: Vec<&str> = invalid.iter().map(|s| s.as_str()).collect();
            return Err(format!(
                "Invalid app modes: {}. Available: {}",
                invalid_str.join(", "),
                available_modes.join(", ")
            ));
        }
        parts
    };

    let client = DifyClient::get_client_with_auth(&url, &email, password).await?;
    let apps = client.list_apps(&modes_to_export, tag.as_deref()).await?;

    if apps.is_empty() {
        if let Some(ref t_name) = tag {
            println!(
                "[!] Safety check: No apps matching tag '{}' were found on the server.",
                t_name
            );
            println!("[!] Aborting export to prevent accidental deletion of local files.");
            return Err("Safety check failed, aborted export.".to_string());
        } else {
            println!("[*] No apps matching criteria to export.");
            return Ok(());
        }
    }

    // Load mapping file
    let map_file_buf = PathBuf::from(&map_file);

    let mut mapping: HashMap<String, HashMap<String, String>> = HashMap::new();
    if map_file_buf.exists()
        && let Ok(content) = fs::read_to_string(&map_file_buf)
    {
        mapping = serde_json::from_str(&content).unwrap_or_default();
    }

    // Ensure the environment entry exists
    let env_map = mapping.entry(env.clone()).or_default();

    // Clean output directories
    for mode in &modes_to_export {
        let folder = get_mode_folder(mode);
        let target_dir = Path::new(&output).join(folder);
        if target_dir.exists() {
            println!("[*] Cleaning up local directory: {:?}", target_dir);
            if let Ok(entries) = fs::read_dir(&target_dir) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if entry_path.is_dir() {
                        let _ = fs::remove_dir_all(&entry_path);
                    } else {
                        let _ = fs::remove_file(&entry_path);
                    }
                }
            }
            // Remove keys from env_map that start with "folder/"
            let prefix = format!("{}/", folder);
            env_map.retain(|k, _| !k.starts_with(&prefix));
        }
    }

    println!(
        "[*] Starting export of {} apps to directory '{}'...",
        apps.len(),
        output
    );

    let mut success_count = 0;
    for (idx, app) in apps.iter().enumerate() {
        println!(
            "[{}/{}] Exporting '{}' (Mode: {}, ID: {})...",
            idx + 1,
            apps.len(),
            app.name,
            app.mode,
            app.id
        );

        match client.export_dsl(&app.id, false).await {
            Ok(dsl_content) => {
                let folder = get_mode_folder(&app.mode);
                let target_dir = Path::new(&output).join(folder);
                if let Err(e) = fs::create_dir_all(&target_dir) {
                    println!("    [!] Failed to create directory: {}", e);
                    continue;
                }

                let safe_name = sanitize_filename(&app.name);
                let filename = format!("{}.yml", safe_name);
                let file_path = target_dir.join(&filename);
                let rel_path = format!("{}/{}", folder, filename);

                if let Err(e) = fs::write(&file_path, &dsl_content) {
                    println!("    [!] Failed to write DSL file {:?}: {}", file_path, e);
                    continue;
                }
                println!("    [+] Saved: {:?}", file_path);

                env_map.insert(rel_path, app.id.clone());
                success_count += 1;
            }
            Err(e) => {
                println!(
                    "    [!] Error exporting '{}' (ID: {}): {}",
                    app.name, app.id, e
                );
            }
        }
    }

    // Save updated mapping file
    if let Some(parent) = map_file_buf.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(serialized) = serde_json::to_string_pretty(&mapping) {
        if let Err(e) = fs::write(&map_file_buf, serialized) {
            println!("[!] Error writing mapping file: {}", e);
        } else {
            println!("[+] Saved updated mapping file: {:?}", map_file_buf);
        }
    }

    println!(
        "\n[+] Export completed: {}/{} successfully exported.",
        success_count,
        apps.len()
    );

    Ok(())
}
