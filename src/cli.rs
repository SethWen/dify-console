use clap::builder::styling::{AnsiColor, Styles};
use clap::{Parser, Subcommand};

fn get_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Green.on_default().bold())
        .usage(AnsiColor::Green.on_default().bold())
        .literal(AnsiColor::Cyan.on_default().bold())
        .placeholder(AnsiColor::Cyan.on_default())
}

#[derive(Parser)]
#[command(name = "dify-console")]
#[command(version)]
#[command(about = "Dify Application DSL CLI Tool", long_about = None)]
#[command(styles = get_styles())]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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
        #[arg(short, long)]
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
