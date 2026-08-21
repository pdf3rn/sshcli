mod app;
mod config;
mod credentials;
mod error;
mod profiles;
mod sftp;
mod ssh;
mod tui;

use clap::{Parser, Subcommand, ValueEnum};
use tracing_subscriber::EnvFilter;

use crate::{
    app::{Action, App},
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
    /// Run an SFTP operation using a saved profile.
    Sftp {
        profile: String,
        #[command(subcommand)]
        command: SftpCommand,
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

#[derive(Debug, Subcommand)]
enum SftpCommand {
    /// List a remote directory.
    Ls {
        #[arg(default_value = ".")]
        path: String,
    },
    /// Print the remote working directory.
    Pwd,
    /// Download a remote file.
    Get {
        remote: String,
        local: std::path::PathBuf,
    },
    /// Upload a local file.
    Put {
        local: std::path::PathBuf,
        remote: String,
    },
    /// Remove a remote file.
    Rm { path: String },
    /// Remove an empty remote directory.
    Rmdir { path: String },
    /// Create a remote directory.
    Mkdir { path: String },
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
        Some(Command::Sftp { profile, command }) => handle_sftp_command(profile, command).await,
        None => run_tui().await,
    }
}

async fn run_tui() -> AppResult<()> {
    let store = ProfileStore::new();
    loop {
        let mut app = App::new(store.load()?);
        let selected = tui::run(&mut app)?;
        let Some(action) = selected else {
            return Ok(());
        };
        match action {
            Action::Connect(profile) => {
                let secret = read_profile_secret(&profile)?;
                ssh::connect_profile(profile, secret).await?;
            }
            Action::Sftp(profile) => {
                let secret = read_profile_secret(&profile)?;
                let options = ssh::options_for_profile(&profile, secret)?;
                let session = ssh::open_sftp(options).await?;
                sftp::browse(session).await?;
            }
        }
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

async fn handle_sftp_command(profile_name: String, command: SftpCommand) -> AppResult<()> {
    let store = ProfileStore::new();
    let profile = store
        .load()?
        .into_iter()
        .find(|profile| profile.name == profile_name)
        .ok_or_else(|| AppError::Profile(format!("profile not found: {profile_name}")))?;
    let secret = read_profile_secret(&profile)?;
    let options = ssh::options_for_profile(&profile, secret)?;
    let session = ssh::open_sftp(options).await?;
    let operation = match command {
        SftpCommand::Ls { path } => sftp::Operation::List { path },
        SftpCommand::Pwd => sftp::Operation::PrintWorkingDirectory,
        SftpCommand::Get { remote, local } => sftp::Operation::Get { remote, local },
        SftpCommand::Put { local, remote } => sftp::Operation::Put { local, remote },
        SftpCommand::Rm { path } => sftp::Operation::RemoveFile { path },
        SftpCommand::Rmdir { path } => sftp::Operation::RemoveDir { path },
        SftpCommand::Mkdir { path } => sftp::Operation::MakeDir { path },
    };
    sftp::execute(session, operation).await
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
