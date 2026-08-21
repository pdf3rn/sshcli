mod app;
mod config;
mod credentials;
mod error;
mod profiles;
mod ssh;
mod tui;

use clap::{Parser, Subcommand, ValueEnum};
use tracing_subscriber::EnvFilter;

use crate::{
    app::App,
    error::{AppError, AppResult},
    profiles::{Authentication, Profile, ProfileStore},
};

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
    /// Manage saved connection profiles.
    Profile {
        #[command(subcommand)]
        command: ProfileCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ProfileCommand {
    /// Save a connection profile. Secrets are requested interactively.
    Add {
        name: String,
        #[arg(long)]
        host: String,
        #[arg(short, long)]
        user: String,
        #[arg(short, long, default_value_t = 22)]
        port: u16,
        #[arg(short = 'i', long)]
        identity_file: Option<String>,
        #[arg(long, value_enum)]
        auth: Option<AuthMode>,
        /// Explicitly accept the server key until known_hosts support is added.
        #[arg(long)]
        accept_unknown_host_key: bool,
    },
    /// List profiles without revealing credentials.
    List,
    /// Remove a profile and its keyring secret.
    Remove { name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, ValueEnum)]
enum AuthMode {
    None,
    Password,
    PrivateKey,
}

impl From<AuthMode> for Authentication {
    fn from(mode: AuthMode) -> Self {
        match mode {
            AuthMode::None => Self::None,
            AuthMode::Password => Self::Password,
            AuthMode::PrivateKey => Self::PrivateKey,
        }
    }
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
            let authentication = if identity_file.is_some() {
                ssh::Authentication::PrivateKey(None)
            } else {
                ssh::Authentication::None
            };
            ssh::connect(ssh::ConnectionOptions {
                host,
                port,
                username: user,
                identity_file,
                accept_unknown_host_key,
                authentication,
            })
            .await
        }
        Some(Command::Profile { command }) => handle_profile_command(command).await,
        None => run_tui().await,
    }
}

async fn run_tui() -> AppResult<()> {
    let store = ProfileStore::new();
    loop {
        let mut app = App::new(store.load()?);
        let selected = tui::run(&mut app)?;
        let Some(profile) = selected else {
            return Ok(());
        };
        let secret = read_profile_secret(&profile)?;
        ssh::connect_profile(profile, secret).await?;
    }
}

async fn handle_profile_command(command: ProfileCommand) -> AppResult<()> {
    let store = ProfileStore::new();
    match command {
        ProfileCommand::List => {
            for profile in store.load()? {
                let auth = match &profile.authentication {
                    Authentication::None => "none",
                    Authentication::Password => "password (keyring)",
                    Authentication::PrivateKey => "private key",
                };
                println!(
                    "{}\t{}@{}:{}\t{}",
                    profile.name, profile.username, profile.host, profile.port, auth
                );
            }
            Ok(())
        }
        ProfileCommand::Remove { name } => {
            store.remove(&name)?;
            let _ = credentials::delete(&name);
            println!("Removed profile: {name}");
            Ok(())
        }
        ProfileCommand::Add {
            name,
            host,
            user,
            port,
            identity_file,
            auth,
            accept_unknown_host_key,
        } => {
            if store.load()?.iter().any(|profile| profile.name == name) {
                return Err(AppError::Profile(format!("profile already exists: {name}")));
            }
            let authentication = auth
                .unwrap_or(if identity_file.is_some() {
                    AuthMode::PrivateKey
                } else {
                    AuthMode::Password
                })
                .into();
            if matches!(&authentication, Authentication::PrivateKey) && identity_file.is_none() {
                return Err(AppError::Profile(
                    "private-key authentication requires --identity-file".into(),
                ));
            }
            let secret = prompt_secret(&authentication)?;
            let profile = Profile {
                name: name.clone(),
                host,
                port,
                username: user,
                identity_file,
                authentication,
                accept_unknown_host_key,
            };
            if let Some(secret) = secret.as_deref() {
                credentials::set(&name, secret)
                    .map_err(|error| AppError::Credential(error.to_string()))?;
            }
            if let Err(error) = store.add(profile) {
                let _ = credentials::delete(&name);
                return Err(error);
            }
            println!("Saved profile: {name}");
            Ok(())
        }
    }
}

fn prompt_secret(authentication: &Authentication) -> AppResult<Option<String>> {
    match authentication {
        Authentication::None => Ok(None),
        Authentication::Password => rpassword::prompt_password("SSH password: ")
            .map(Some)
            .map_err(AppError::from),
        Authentication::PrivateKey => {
            let passphrase = rpassword::prompt_password("Key passphrase (leave empty if none): ")?;
            if passphrase.is_empty() {
                Ok(None)
            } else {
                Ok(Some(passphrase))
            }
        }
    }
}

fn read_profile_secret(profile: &Profile) -> AppResult<Option<String>> {
    match &profile.authentication {
        Authentication::None => Ok(None),
        Authentication::Password => credentials::get(&profile.name)
            .map(Some)
            .map_err(|error| AppError::Credential(error.to_string())),
        Authentication::PrivateKey => Ok(credentials::get(&profile.name).ok()),
    }
}
