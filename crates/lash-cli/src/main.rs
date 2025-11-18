//! Lash - Minimalist Markdown-native task tracker
//!
//! Command-line interface for the Lash task management system.

#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "lash",
    version,
    about = "Minimalist Markdown-native task tracker",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Parser, Debug)]
enum Commands {
    /// Display version information
    Version,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Version) | None => {
            println!("lash {}", env!("CARGO_PKG_VERSION"));
        }
    }
}
