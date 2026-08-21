mod app;
mod config;
mod error;
mod tui;

use clap::Parser;
use tracing_subscriber::EnvFilter;

use crate::{app::App, error::AppResult};

#[derive(Debug, Parser)]
#[command(
    name = "sshcli",
    version,
    about = "A cross-platform SSH client for the terminal"
)]
struct Cli {
    /// Print the platform-specific configuration directory and exit.
    #[arg(long)]
    print_config_dir: bool,
}

fn main() -> AppResult<()> {
    let cli = Cli::parse();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_target(false)
        .try_init()
        .ok();

    if cli.print_config_dir {
        println!("{}", config::config_dir().display());
        return Ok(());
    }

    let mut app = App::default();
    tui::run(&mut app)
}
