use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("terminal error: {0}")]
    Terminal(#[from] std::io::Error),
    #[error("ssh error: {0}")]
    Ssh(#[from] russh::Error),
    #[error("key error: {0}")]
    Key(#[from] russh::keys::Error),
    #[error("ssh authentication failed")]
    AuthenticationFailed,
    #[error("profile error: {0}")]
    Profile(String),
    #[error("credential error: {0}")]
    Credential(String),
}

pub type AppResult<T> = Result<T, AppError>;
