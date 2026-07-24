mod cli;
mod commands;
mod dify_api;
mod session;
mod utils;

use clap::Parser;
use cli::Cli;

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    if let Err(e) = commands::run(args).await {
        eprintln!("[!] Error: {}", e);
        std::process::exit(1);
    }
}
