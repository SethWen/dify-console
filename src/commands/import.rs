use crate::dify_api::DifyClient;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[allow(clippy::too_many_arguments)]
pub async fn run(
    url: String,
    email: String,
    password: Option<String>,
    file: Option<String>,
    dir: Option<String>,
    app_id: Option<String>,
    map_file: String,
    env: String,
    publish: bool,
) -> Result<(), String> {
    if file.is_none() && dir.is_none() {
        return Err("At least one of -f/--file or -d/--dir must be specified.".to_string());
    }
    if file.is_some() && dir.is_some() {
        return Err("Only one of -f/--file or -d/--dir can be specified.".to_string());
    }

    let map_file_buf = PathBuf::from(&map_file);
    let map_file_exists = map_file_buf.exists();
    let mut mapping: HashMap<String, crate::utils::EnvConfig> = HashMap::new();

    if map_file_exists {
        let content = fs::read_to_string(&map_file_buf)
            .map_err(|e| format!("Failed to read mapping file {:?}: {}", map_file_buf, e))?;
        mapping = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse mapping JSON in {:?}: {}", map_file_buf, e))?;
        println!("[*] Loaded mappings from {:?}", map_file_buf);
    }

    let env_config = mapping
        .entry(env.clone())
        .or_insert_with(|| crate::utils::EnvConfig {
            apps: HashMap::new(),
            replace_rules: None,
        });

    let client = DifyClient::get_client_with_auth(&url, &email, password).await?;

    if let Some(file_path) = file {
        if !Path::new(&file_path).exists() {
            return Err(format!("File not found: {}", file_path));
        }

        println!(
            "[*] Importing DSL to create/update app (Target App ID: {:?})...",
            app_id
        );
        let mut yaml_content =
            fs::read_to_string(&file_path).map_err(|e| format!("Failed to read file: {}", e))?;

        if let Some(ref rules) = env_config.replace_rules
            && !rules.is_empty()
        {
            println!("[*] Applying {} replacement rules...", rules.len());
            yaml_content = crate::utils::replace_yaml_content(&yaml_content, rules)?;
        }

        match client.import_dsl(&yaml_content, app_id.as_deref()).await {
            Ok(result) => {
                let (fallback_name, fallback_mode) =
                    crate::utils::parse_dsl_metadata(&yaml_content);
                let res_name = result
                    .name
                    .or(fallback_name)
                    .unwrap_or_else(|| "Unknown".to_string());
                let res_id = result
                    .app_id
                    .or(result.id)
                    .unwrap_or_else(|| "Unknown".to_string());
                let res_mode = result
                    .mode
                    .or(fallback_mode)
                    .unwrap_or_else(|| "Unknown".to_string());
                println!("[+] Import successful!");
                println!("    - App Name: {}", res_name);
                println!("    - App ID: {}", res_id);
                println!("    - Mode: {}", res_mode);

                if publish {
                    if res_mode == "workflow" || res_mode == "advanced-chat" {
                        println!("[*] Publishing app {}...", res_id);
                        if let Err(e) = client.publish_workflow(&res_id).await {
                            return Err(format!("Publish failed: {}", e));
                        }
                        println!("[+] Published successfully!");
                    } else {
                        println!(
                            "[*] For non-workflow app (mode: {}), skipped automatic publishing.",
                            res_mode
                        );
                    }
                }
            }
            Err(e) => {
                return Err(format!("Import failed: {}", e));
            }
        }
    } else if let Some(dir_path) = dir {
        let dir_path_buf = PathBuf::from(&dir_path);
        if !dir_path_buf.exists() {
            return Err(format!("Directory not found: {}", dir_path));
        }

        // Gather non-symlink yml/yaml files
        let mut yml_files = Vec::new();
        for entry in WalkDir::new(&dir_path_buf)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            let metadata = entry.path().symlink_metadata();

            let is_symlink = match metadata {
                Ok(meta) => meta.file_type().is_symlink(),
                Err(_) => false,
            };

            if is_symlink {
                continue;
            }

            if path.is_file()
                && let Some(ext) = path.extension().and_then(|e| e.to_str())
                && (ext == "yml" || ext == "yaml")
            {
                yml_files.push(path.to_path_buf());
            }
        }

        if yml_files.is_empty() {
            println!(
                "[*] No DSL files found in {:?} (excluding symlinks).",
                dir_path_buf
            );
            return Ok(());
        }

        yml_files.sort();
        println!("[*] Found {} files to import.", yml_files.len());
        let mut success_count = 0;

        for (idx, path) in yml_files.iter().enumerate() {
            let rel_path = match path.strip_prefix(&dir_path_buf) {
                Ok(p) => p.to_string_lossy().into_owned(),
                Err(_) => path.to_string_lossy().into_owned(),
            };
            // Normalize separator to '/'
            let rel_path = rel_path.replace('\\', "/");

            println!(
                "\n[{}/{}] Processing '{}'...",
                idx + 1,
                yml_files.len(),
                rel_path
            );

            let mut yaml_content = match fs::read_to_string(path) {
                Ok(content) => content,
                Err(e) => {
                    println!("    [!] Error reading file: {}", e);
                    continue;
                }
            };

            if let Some(ref rules) = env_config.replace_rules
                && !rules.is_empty()
            {
                println!("    [*] Applying {} replacement rules...", rules.len());
                match crate::utils::replace_yaml_content(&yaml_content, rules) {
                    Ok(replaced) => yaml_content = replaced,
                    Err(e) => {
                        println!("    [!] Failed to apply replacement rules: {}", e);
                        continue;
                    }
                }
            }

            let target_app_id = env_config.apps.get(&rel_path).cloned();
            if let Some(target_app_id) = &target_app_id {
                println!("    [*] Found target mapping App ID: {}", target_app_id);
            } else {
                if map_file_exists {
                    println!(
                        "    [*] No target mapping found in environment '{}' for '{}'. Skipping import to prevent creation of a new app.",
                        env, rel_path
                    );
                    continue;
                }
                println!("    [*] No target mapping. Importing as a new app.");
            }

            match client
                .import_dsl(&yaml_content, target_app_id.as_deref())
                .await
            {
                Ok(result) => {
                    let (fallback_name, fallback_mode) =
                        crate::utils::parse_dsl_metadata(&yaml_content);
                    let res_name = result
                        .name
                        .or(fallback_name)
                        .unwrap_or_else(|| "Unknown".to_string());
                    let res_id = result
                        .app_id
                        .or(result.id)
                        .unwrap_or_else(|| "Unknown".to_string());
                    let res_mode = result
                        .mode
                        .or(fallback_mode)
                        .unwrap_or_else(|| "Unknown".to_string());

                    success_count += 1;
                    println!("    [+] Import successful!");
                    println!("        - App Name: {}", res_name);
                    println!("        - App ID: {}", res_id);
                    println!("        - Mode: {}", res_mode);

                    // Update mapping if it was newly created or changed
                    if target_app_id.as_ref() != Some(&res_id) {
                        env_config.apps.insert(rel_path.clone(), res_id.clone());
                        println!("        [+] Updated mapping: {} -> {}", rel_path, res_id);
                    }

                    if publish {
                        if res_mode == "workflow" || res_mode == "advanced-chat" {
                            println!("        [*] Publishing app {}...", res_id);
                            match client.publish_workflow(&res_id).await {
                                Ok(_) => println!("        [+] Published successfully!"),
                                Err(e) => {
                                    println!("        [!] Publish failed for {}: {}", res_id, e)
                                }
                            }
                        } else {
                            println!(
                                "        [*] For non-workflow app (mode: {}), skipped automatic publishing.",
                                res_mode
                            );
                        }
                    }
                }
                Err(e) => {
                    println!("    [!] Failed to import '{}': {}", rel_path, e);
                }
            }
        }

        // Write the mapping file once at the end, only if it did not exist initially
        if !map_file_exists {
            if let Ok(serialized) = serde_json::to_string_pretty(&mapping) {
                if let Err(e) = fs::write(&map_file_buf, serialized) {
                    println!("    [!] Error writing mapping file: {}", e);
                } else {
                    println!("    [+] Saved updated mapping file: {:?}", map_file_buf);
                }
            }
        } else {
            println!(
                "    [*] Mapping file already exists. Skipping saving changes to prevent modification."
            );
        }

        println!(
            "\n[+] Batch import completed: {}/{} imported successfully.",
            success_count,
            yml_files.len()
        );

        if success_count != yml_files.len() {
            return Err("Some files failed to import.".to_string());
        }
    }

    Ok(())
}
