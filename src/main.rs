mod dify_api;
mod session;

use clap::{Parser, Subcommand};
use dify_api::DifyClient;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "dify-console")]
#[command(about = "Dify Application DSL CLI Tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Export applications from Dify WebUI to local DSL files
    Export {
        /// Dify Console Base URL (e.g. http://localhost:8080 or https://dify.example.com)
        #[arg(short, long)]
        url: String,

        /// Dify Console User/Admin Email
        #[arg(short, long)]
        email: String,

        /// Dify Console Password (if omitted, you will be prompted in the terminal)
        #[arg(short, long)]
        password: Option<String>,

        /// Output directory to save exported DSL files
        #[arg(short, long, default_value = "difydsl")]
        output: String,

        /// Comma-separated list of app modes to export. Available: workflow, advanced-chat, chat, agent-chat, completion. Or 'all' to export all modes.
        #[arg(short, long, default_value = "workflow,advanced-chat")]
        modes: String,

        /// Filter apps by a specific tag name
        #[arg(short, long, default_value = "智能投标")]
        tag: Option<String>,

        /// Environment name for mapping (e.g. dev, preview, production)
        #[arg(short = 'E', long, default_value = "dev")]
        env: String,

        /// Path to the app ID mapping JSON file
        #[arg(long, default_value = "difydsl/app_mapping.json")]
        map_file: String,
    },

    /// Import local DSL files back to Dify WebUI
    Import {
        /// Dify Console Base URL (e.g. http://localhost:8080 or https://dify.example.com)
        #[arg(short, long)]
        url: String,

        /// Dify Console User/Admin Email
        #[arg(short, long)]
        email: String,

        /// Dify Console Password (if omitted, you will be prompted in the terminal)
        #[arg(short, long)]
        password: Option<String>,

        /// Path to a single DSL yml file to import
        #[arg(short, long)]
        file: Option<String>,

        /// Path to a directory containing DSL files for batch import
        #[arg(short, long)]
        dir: Option<String>,

        /// Optional target App ID (only applicable when importing a single file -f)
        #[arg(short, long)]
        app_id: Option<String>,

        /// Path to the app ID mapping JSON file (only applicable when batch importing -d)
        #[arg(short, long, default_value = "difydsl/app_mapping.json")]
        map_file: String,

        /// Environment name for mapping (e.g. dev, preview, production)
        #[arg(short = 'E', long)]
        env: String,
    },
}

fn sanitize_filename(name: &str) -> String {
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

fn get_mode_folder(mode: &str) -> &str {
    match mode {
        "workflow" => "workflows",
        "advanced-chat" => "chatflows",
        _ => mode,
    }
}

async fn get_client_with_auth(
    url: &str,
    email: &str,
    password: Option<String>,
) -> Result<DifyClient, String> {
    // 1. Try to load cached session
    if let Some(cached) = session::get_cached_session(url, email) {
        println!(
            "[*] Found cached session for {} ({}). Checking validity...",
            email, url
        );
        let client = DifyClient::with_session(url, &cached.cookies, &cached.csrf_token);
        if client.check_session().await {
            println!("[+] Cached session is valid.");
            return Ok(client);
        }
        println!("[!] Cached session expired or invalid.");
    }

    // 2. No valid session, authenticate
    let pwd = match password {
        Some(p) => p,
        None => {
            let prompt = format!("Enter password for Dify Console ({}): ", email);
            rpassword::prompt_password(prompt)
                .map_err(|e| format!("Failed to read password: {}", e))?
        }
    };

    let mut client = DifyClient::new(url);
    let (cookies, csrf_token) = client.login(email, &pwd).await?;

    // 3. Cache session
    if let Err(e) = session::save_session(url, email, &cookies, &csrf_token) {
        println!("[!] Warning: Failed to save session to cache: {}", e);
    } else {
        println!("[+] Session cached successfully.");
    }

    Ok(client)
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    if let Err(e) = run(args).await {
        eprintln!("[!] Error: {}", e);
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Commands::Export {
            url,
            email,
            password,
            output,
            modes,
            tag,
            env,
            map_file,
        } => {
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

            let client = get_client_with_auth(&url, &email, password).await?;
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
            if map_file_buf.exists() {
                if let Ok(content) = fs::read_to_string(&map_file_buf) {
                    mapping = serde_json::from_str(&content).unwrap_or_default();
                }
            }

            // Ensure the environment entry exists
            let env_map = mapping.entry(env.clone()).or_insert_with(HashMap::new);

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
        }

        Commands::Import {
            url,
            email,
            password,
            file,
            dir,
            app_id,
            map_file,
            env,
        } => {
            if file.is_none() && dir.is_none() {
                return Err("At least one of -f/--file or -d/--dir must be specified.".to_string());
            }
            if file.is_some() && dir.is_some() {
                return Err("Only one of -f/--file or -d/--dir can be specified.".to_string());
            }

            let client = get_client_with_auth(&url, &email, password).await?;

            if let Some(file_path) = file {
                if !Path::new(&file_path).exists() {
                    return Err(format!("File not found: {}", file_path));
                }

                println!(
                    "[*] Importing DSL to create/update app (Target App ID: {:?})...",
                    app_id
                );
                let yaml_content = fs::read_to_string(&file_path)
                    .map_err(|e| format!("Failed to read file: {}", e))?;

                match client.import_dsl(&yaml_content, app_id.as_deref()).await {
                    Ok(result) => {
                        let res_name = result.name.unwrap_or_else(|| "Unknown".to_string());
                        let res_id = result
                            .app_id
                            .or(result.id)
                            .unwrap_or_else(|| "Unknown".to_string());
                        let res_mode = result.mode.unwrap_or_else(|| "Unknown".to_string());
                        println!("[+] Import successful!");
                        println!("    - App Name: {}", res_name);
                        println!("    - App ID: {}", res_id);
                        println!("    - Mode: {}", res_mode);
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

                let map_file_buf = PathBuf::from(&map_file);
                let mut mapping: HashMap<String, HashMap<String, String>> = HashMap::new();

                if map_file_buf.exists() {
                    match fs::read_to_string(&map_file_buf) {
                        Ok(content) => {
                            mapping = serde_json::from_str(&content).unwrap_or_default();
                            println!("[*] Loaded mappings from {:?}", map_file_buf);
                        }
                        Err(e) => {
                            println!(
                                "[!] Warning: Failed to load mapping file {:?}: {}",
                                map_file_buf, e
                            );
                        }
                    }
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

                    if path.is_file() {
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            if ext == "yml" || ext == "yaml" {
                                yml_files.push(path.to_path_buf());
                            }
                        }
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

                // Ensure the environment entry exists
                let env_map = mapping.entry(env.clone()).or_insert_with(HashMap::new);

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

                    let yaml_content = match fs::read_to_string(path) {
                        Ok(content) => content,
                        Err(e) => {
                            println!("    [!] Error reading file: {}", e);
                            continue;
                        }
                    };

                    let target_app_id = env_map.get(&rel_path).cloned();
                    if target_app_id.is_some() {
                        println!(
                            "    [*] Found target mapping App ID: {}",
                            target_app_id.as_ref().unwrap()
                        );
                    } else {
                        println!("    [*] No target mapping. Importing as a new app.");
                    }

                    match client
                        .import_dsl(&yaml_content, target_app_id.as_deref())
                        .await
                    {
                        Ok(result) => {
                            let res_name = result.name.unwrap_or_else(|| "Unknown".to_string());
                            let res_id = result
                                .app_id
                                .or(result.id)
                                .unwrap_or_else(|| "Unknown".to_string());
                            let res_mode = result.mode.unwrap_or_else(|| "Unknown".to_string());

                            success_count += 1;
                            println!("    [+] Import successful!");
                            println!("        - App Name: {}", res_name);
                            println!("        - App ID: {}", res_id);
                            println!("        - Mode: {}", res_mode);

                            // Update mapping if it was newly created or changed
                            if target_app_id.as_ref() != Some(&res_id) {
                                env_map.insert(rel_path.clone(), res_id.clone());
                                println!("        [+] Updated mapping: {} -> {}", rel_path, res_id);
                            }
                        }
                        Err(e) => {
                            println!("    [!] Failed to import '{}': {}", rel_path, e);
                        }
                    }
                }

                // Write the mapping file once at the end
                if let Ok(serialized) = serde_json::to_string_pretty(&mapping) {
                    if let Err(e) = fs::write(&map_file_buf, serialized) {
                        println!("    [!] Error writing mapping file: {}", e);
                    } else {
                        println!("    [+] Saved updated mapping file: {:?}", map_file_buf);
                    }
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
        }
    }

    Ok(())
}
