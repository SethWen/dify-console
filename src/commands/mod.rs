pub mod export;
pub mod import;

use crate::cli::{Cli, Commands};

pub async fn run(cli: Cli) -> Result<(), String> {
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
        } => export::run(url, email, password, output, modes, tag, env, map_file).await,
        Commands::Import {
            url,
            email,
            password,
            file,
            dir,
            app_id,
            map_file,
            env,
            publish,
        } => {
            import::run(
                url, email, password, file, dir, app_id, map_file, env, publish,
            )
            .await
        }
    }
}
