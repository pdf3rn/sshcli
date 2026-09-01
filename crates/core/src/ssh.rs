use std::{sync::Arc, time::Duration};

use russh::{client, ChannelMsg};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Notify;
use tokio::task::{JoinHandle, JoinSet};

use crate::{
    error::{AppError, AppResult},
    profiles::{Authentication as ProfileAuthentication, Profile},
};

pub struct ClientHandler {
    host: String,
    port: u16,
    accept_unknown_host_key: bool,
}

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(crate::host_keys::verify_or_add(
            &self.host,
            self.port,
            &server_public_key.to_string(),
            self.accept_unknown_host_key,
        )
        .unwrap_or(false))
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

fn default_identity_file() -> Option<String> {
    let home = directories::BaseDirs::new()?.home_dir().to_path_buf();
    ["id_ed25519", "id_ecdsa", "id_rsa"]
        .into_iter()
        .map(|name| home.join(".ssh").join(name))
        .find(|path| path.exists())
        .map(|path| path.to_string_lossy().into_owned())
}

pub fn options_adhoc(
    host: String,
    port: u16,
    username: String,
    password: Option<String>,
) -> AppResult<ConnectionOptions> {
    let authentication;
    let mut identity_file = None;
    match password {
        Some(password) => authentication = Authentication::Password(password),
        None => {
            let key_path = default_identity_file().ok_or_else(|| {
                AppError::Credential(
                    "sin contraseña y sin clave por defecto en ~/.ssh; indica una contraseña"
                        .into(),
                )
            })?;
            identity_file = Some(key_path);
            authentication = Authentication::PrivateKey(None);
        }
    }

    Ok(ConnectionOptions {
        host,
        port,
        username,
        identity_file,
        accept_unknown_host_key: true,
        authentication,
    })
}

pub async fn open_shell(
    options: ConnectionOptions,
    columns: u16,
    rows: u16,
) -> AppResult<(
    russh::Channel<russh::client::Msg>,
    client::Handle<ClientHandler>,
)> {
    let session = authenticate(options).await?;
    let channel = session.channel_open_session().await?;
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
    Ok((channel, session))
}

pub async fn open_sftp(options: ConnectionOptions) -> AppResult<russh_sftp::client::SftpSession> {
    let session = authenticate(options).await?;
    let channel = session.channel_open_session().await?;
    channel.request_subsystem(true, "sftp").await?;
    russh_sftp::client::SftpSession::new(channel.into_stream())
        .await
        .map_err(|error| AppError::Sftp(error.to_string()))
}

pub struct ExecSession {
    handle: client::Handle<ClientHandler>,
}

pub async fn collect_exec(
    handle: &client::Handle<ClientHandler>,
    command: &str,
) -> AppResult<String> {
    let mut channel = handle.channel_open_session().await?;
    channel.exec(true, command).await?;
    let mut output = Vec::new();
    while let Some(message) = channel.wait().await {
        match message {
            ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => {
                output.extend_from_slice(&data);
            }
            ChannelMsg::Eof | ChannelMsg::Close => break,
            _ => {}
        }
    }
    Ok(String::from_utf8_lossy(&output).into_owned())
}

impl ExecSession {
    pub async fn connect(options: ConnectionOptions) -> AppResult<Self> {
        Ok(Self {
            handle: authenticate(options).await?,
        })
    }

    pub fn is_closed(&self) -> bool {
        self.handle.is_closed()
    }

    pub async fn run(&mut self, command: &str) -> AppResult<String> {
        collect_exec(&self.handle, command).await
    }

    pub async fn disconnect(self) {}
}

pub struct LocalForward {
    pub bind_addr: std::net::SocketAddr,
    pub target_host: String,
    pub target_port: u16,
    stop: Arc<Notify>,
    join: JoinHandle<AppResult<()>>,
}

impl LocalForward {
    pub async fn start(
        options: ConnectionOptions,
        bind_host: String,
        bind_port: u16,
        target_host: String,
        target_port: u16,
    ) -> AppResult<Self> {
        let listener = TcpListener::bind((bind_host.as_str(), bind_port)).await?;
        let bind_addr = listener.local_addr()?;
        let session = Arc::new(authenticate(options).await?);
        let stop = Arc::new(Notify::new());
        let stop_task = stop.clone();
        let target = target_host.clone();
        let join = tokio::spawn(async move {
            let mut forwards = JoinSet::new();
            loop {
                tokio::select! {
                    connection = listener.accept() => {
                        let (socket, origin) = connection?;
                        let session = session.clone();
                        let target = target.clone();
                        forwards.spawn(async move {
                            let _ = forward_connection(&session, socket, origin, &target, target_port).await;
                        });
                    }
                    _ = stop_task.notified() => {
                        forwards.abort_all();
                        while forwards.join_next().await.is_some() {}
                        break;
                    }
                    _ = forwards.join_next(), if !forwards.is_empty() => {}
                }
            }
            Ok(())
        });
        Ok(Self {
            bind_addr,
            target_host,
            target_port,
            stop,
            join,
        })
    }

    pub async fn stop(self) {
        self.stop.notify_one();
        let _ = self.join.await;
    }
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
        keepalive_interval: Some(Duration::from_secs(30)),
        keepalive_max: 3,
        nodelay: true,
        ..Default::default()
    };
    let address = (options.host.as_str(), options.port);
    let mut session = client::connect(
        Arc::new(config),
        address,
        ClientHandler {
            host: options.host.clone(),
            port: options.port,
            accept_unknown_host_key: options.accept_unknown_host_key,
        },
    )
    .await?;

    let key_path = options
        .identity_file
        .as_deref()
        .map(crate::keys::expand_home);
    let key = match key_path {
        Some(path) => match &options.authentication {
            Authentication::PrivateKey(passphrase) => {
                Some(russh::keys::load_secret_key(&path, passphrase.as_deref())?)
            }
            _ => None,
        },
        None => None,
    };

    let authenticated = match options.authentication {
        Authentication::Password(password) => authenticate_password(&mut session, options.username, password).await?,
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
                .success()
        }
        Authentication::None => session.authenticate_none(options.username).await?.success(),
    };
    if !authenticated {
        return Err(crate::error::AppError::AuthenticationFailed);
    }
    Ok(session)
}

async fn authenticate_password(
    session: &mut client::Handle<ClientHandler>,
    username: String,
    password: String,
) -> AppResult<bool> {
    if session
        .authenticate_password(username.clone(), password.clone())
        .await?
        .success()
    {
        return Ok(true);
    }

    let mut response = session
        .authenticate_keyboard_interactive_start(username, None)
        .await?;
    for _ in 0..8 {
        response = match response {
            client::KeyboardInteractiveAuthResponse::Success => return Ok(true),
            client::KeyboardInteractiveAuthResponse::Failure { .. } => return Ok(false),
            client::KeyboardInteractiveAuthResponse::InfoRequest { prompts, .. } => {
                if prompts.iter().any(|prompt| prompt.echo) {
                    return Ok(false);
                }
                session
                    .authenticate_keyboard_interactive_respond(
                        prompts.into_iter().map(|_| password.clone()).collect(),
                    )
                    .await?
            }
        };
    }
    Ok(false)
}
