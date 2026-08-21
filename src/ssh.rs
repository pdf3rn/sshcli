use std::{path::Path, sync::Arc, time::Duration};

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use russh::{client, ChannelMsg};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

use crate::error::AppResult;

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
}

pub async fn connect(options: ConnectionOptions) -> AppResult<()> {
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
        Some(path) => Some(russh::keys::load_secret_key(path, None)?),
        None => None,
    };

    let authenticated = match key {
        Some(key) => {
            session
                .authenticate_publickey(
                    options.username,
                    russh::keys::PrivateKeyWithHashAlg::new(Arc::new(key), None),
                )
                .await?
        }
        None => session.authenticate_none(options.username).await?,
    };
    if !authenticated.success() {
        return Err(crate::error::AppError::AuthenticationFailed);
    }

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
