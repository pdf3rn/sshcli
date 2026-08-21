use std::{path::Path, sync::Arc, time::Duration};

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use russh::{client, ChannelMsg};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::{
    error::{AppError, AppResult},
    profiles::{Authentication as ProfileAuthentication, Profile},
};

struct ClientHandler {
    accept_unknown_host_key: bool,
}

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(self.accept_unknown_host_key)
    }
}

pub struct ConnectionOptions {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub identity_file: Option<String>,
    pub accept_unknown_host_key: bool,
    pub authentication: Authentication,
}

pub enum Authentication {
    None,
    Password(String),
    PrivateKey(Option<String>),
}

pub fn options_for_profile(
    profile: &Profile,
    secret: Option<String>,
) -> AppResult<ConnectionOptions> {
    let authentication = match &profile.authentication {
        ProfileAuthentication::None => Authentication::None,
        ProfileAuthentication::Password => Authentication::Password(secret.ok_or_else(|| {
            AppError::Credential(format!("missing password for profile {}", profile.name))
        })?),
        ProfileAuthentication::PrivateKey => Authentication::PrivateKey(secret),
    };

    Ok(ConnectionOptions {
        host: profile.host.clone(),
        port: profile.port,
        username: profile.username.clone(),
        identity_file: profile.identity_file.clone(),
        accept_unknown_host_key: profile.accept_unknown_host_key,
        authentication,
    })
}

pub async fn connect(options: ConnectionOptions) -> AppResult<()> {
    let session = authenticate(options).await?;
    let channel = session.channel_open_session().await?;
    channel
        .request_pty(true, "xterm-256color", 120, 40, 0, 0, &[])
        .await?;
    channel.request_shell(true).await?;

    enable_raw_mode()?;
    let result = run_terminal(channel).await;
    disable_raw_mode()?;
    result
}

pub async fn open_shell(
    options: ConnectionOptions,
) -> AppResult<russh::Channel<russh::client::Msg>> {
    let session = authenticate(options).await?;
    let channel = session.channel_open_session().await?;
    let (columns, rows) = crossterm::terminal::size().unwrap_or((120, 40));
    channel
        .request_pty(
            true,
            "xterm-256color",
            u32::from(columns),
            u32::from(rows),
            0,
            0,
            &[],
        )
        .await?;
    channel.request_shell(true).await?;
    Ok(channel)
}

pub async fn open_sftp(options: ConnectionOptions) -> AppResult<russh_sftp::client::SftpSession> {
    let session = authenticate(options).await?;
    let channel = session.channel_open_session().await?;
    channel.request_subsystem(true, "sftp").await?;
    russh_sftp::client::SftpSession::new(channel.into_stream())
        .await
        .map_err(|error| AppError::Sftp(error.to_string()))
}

pub async fn forward_local(
    options: ConnectionOptions,
    bind_host: String,
    bind_port: u16,
    target_host: String,
    target_port: u16,
) -> AppResult<()> {
    let listener = TcpListener::bind((bind_host.as_str(), bind_port)).await?;
    let local_address = listener.local_addr()?;
    println!("Forwarding {local_address} -> {target_host}:{target_port}. Press Ctrl-C to stop.");
    let session = authenticate(options).await?;

    loop {
        tokio::select! {
            connection = listener.accept() => {
                let (socket, origin) = connection?;
                forward_connection(&session, socket, origin, &target_host, target_port).await?;
            }
            _ = tokio::signal::ctrl_c() => break,
        }
    }
    Ok(())
}

async fn forward_connection(
    session: &client::Handle<ClientHandler>,
    mut socket: TcpStream,
    origin: std::net::SocketAddr,
    target_host: &str,
    target_port: u16,
) -> AppResult<()> {
    let channel = session
        .channel_open_direct_tcpip(
            target_host,
            u32::from(target_port),
            origin.ip().to_string(),
            u32::from(origin.port()),
        )
        .await?;
    let mut remote = channel.into_stream();
    tokio::io::copy_bidirectional(&mut socket, &mut remote).await?;
    Ok(())
}

async fn authenticate(options: ConnectionOptions) -> AppResult<client::Handle<ClientHandler>> {
    let config = client::Config {
        inactivity_timeout: Some(Duration::from_secs(60 * 60)),
        ..Default::default()
    };
    let address = (options.host.as_str(), options.port);
    let mut session = client::connect(
        Arc::new(config),
        address,
        ClientHandler {
            accept_unknown_host_key: options.accept_unknown_host_key,
        },
    )
    .await?;

    let key_path = options.identity_file.as_deref().map(Path::new);
    let key = match key_path {
        Some(path) => match &options.authentication {
            Authentication::PrivateKey(passphrase) => {
                Some(russh::keys::load_secret_key(path, passphrase.as_deref())?)
            }
            _ => None,
        },
        None => None,
    };

    let authenticated = match options.authentication {
        Authentication::Password(password) => {
            session
                .authenticate_password(options.username, password)
                .await?
        }
        Authentication::PrivateKey(_) => {
            let key = key.ok_or_else(|| {
                AppError::Profile("private-key authentication requires an identity file".into())
            })?;
            session
                .authenticate_publickey(
                    options.username,
                    russh::keys::PrivateKeyWithHashAlg::new(Arc::new(key), None),
                )
                .await?
        }
        Authentication::None => session.authenticate_none(options.username).await?,
    };
    if !authenticated.success() {
        return Err(crate::error::AppError::AuthenticationFailed);
    }
    Ok(session)
}

async fn run_terminal(mut channel: russh::Channel<russh::client::Msg>) -> AppResult<()> {
    let mut input = io::stdin();
    let mut output = io::stdout();
    let mut input_buffer = [0_u8; 4096];

    loop {
        tokio::select! {
            read = input.read(&mut input_buffer) => {
                let count = read?;
                if count == 0 {
                    break;
                }
                channel.data(&input_buffer[..count]).await?;
            }
            message = channel.wait() => {
                match message {
                    Some(ChannelMsg::Data { data }) | Some(ChannelMsg::ExtendedData { data, .. }) => {
                        output.write_all(&data).await?;
                        output.flush().await?;
                    }
                    Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | None => break,
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
