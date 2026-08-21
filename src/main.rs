mod app;
mod config;
mod error;
mod ssh;
mod tui;

use clap::{Parser, Subcommand};
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

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Open an interactive SSH session.
    Connect {
        host: String,
        #[arg(short, long, default_value_t = 22)]
        port: u16,
        #[arg(short, long)]
        user: String,
        #[arg(short = 'i', long)]
        identity_file: Option<String>,
        /// Explicitly accept the server key until known_hosts support is added.
        #[arg(long)]
        accept_unknown_host_key: bool,
    },
}

#[tokio::main]
async fn main() -> AppResult<()> {
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

    match cli.command {
        Some(Command::Connect {
            host,
            port,
            user,
            identity_file,
            accept_unknown_host_key,
        }) => {
            ssh::connect(ssh::ConnectionOptions {
                host,
                port,
                username: user,
                identity_file,
                accept_unknown_host_key,
            })
            .await
        }
        None => {
            let mut app = App::default();
            tui::run(&mut app)
        }
    }
}
