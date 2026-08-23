use std::path::PathBuf;

use russh_sftp::{
    client::SftpSession,
    protocol::{FileType, OpenFlags},
};
use tokio::{
    fs::File,
    io::{self, AsyncWriteExt},
};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct RemoteEntry {
    pub name: String,
    pub kind: FileType,
    pub size: u64,
}

pub async fn list_dir(session: &SftpSession, path: &str) -> AppResult<Vec<RemoteEntry>> {
    let entries = session
        .read_dir(path)
        .await
        .map_err(|error| AppError::Sftp(error.to_string()))?;
    Ok(entries
        .map(|entry| RemoteEntry {
            name: entry.file_name(),
            kind: entry.file_type(),
            size: entry.metadata().size.unwrap_or(0),
        })
        .collect())
}

pub async fn download(session: &SftpSession, remote: &str, local: &PathBuf) -> AppResult<()> {
    let mut source = session
        .open(remote)
        .await
        .map_err(|error| AppError::Sftp(error.to_string()))?;
    let mut destination = File::create(local).await?;
    io::copy(&mut source, &mut destination).await?;
    destination.flush().await?;
    Ok(())
}

pub async fn upload(session: &SftpSession, local: &PathBuf, remote: &str) -> AppResult<()> {
    let mut source = File::open(local).await?;
    let mut destination = session
        .open_with_flags(
            remote,
            OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE,
        )
        .await
        .map_err(|error| AppError::Sftp(error.to_string()))?;
    io::copy(&mut source, &mut destination).await?;
    destination.flush().await?;
    destination
        .shutdown()
        .await
        .map_err(|error| AppError::Sftp(error.to_string()))?;
    Ok(())
}

pub async fn remove_file(session: &SftpSession, path: &str) -> AppResult<()> {
    session
        .remove_file(path)
        .await
        .map_err(|error| AppError::Sftp(error.to_string()))
}

pub async fn remove_dir(session: &SftpSession, path: &str) -> AppResult<()> {
    session
        .remove_dir(path)
        .await
        .map_err(|error| AppError::Sftp(error.to_string()))
}

pub async fn create_dir(session: &SftpSession, path: &str) -> AppResult<()> {
    session
        .create_dir(path)
        .await
        .map_err(|error| AppError::Sftp(error.to_string()))
}

pub async fn canonicalize(session: &SftpSession, path: &str) -> AppResult<String> {
    session
        .canonicalize(path)
        .await
        .map_err(|error| AppError::Sftp(error.to_string()))
}

pub fn join_remote(parent: &str, child: &str) -> String {
    if parent == "." {
        child.to_string()
    } else if parent == "/" {
        format!("/{child}")
    } else {
        format!("{parent}/{child}")
    }
}

pub fn parent_path(path: &str) -> String {
    path.rsplit_once('/')
        .map(|(parent, _)| {
            if parent.is_empty() {
                "/".into()
            } else {
                parent.into()
            }
        })
        .unwrap_or_else(|| ".".into())
}

#[cfg(test)]
mod tests {
    use super::{join_remote, parent_path};

    #[test]
    fn remote_paths_are_joined_without_duplicate_separators() {
        assert_eq!(join_remote(".", "file.txt"), "file.txt");
        assert_eq!(join_remote("/var/log", "app.log"), "/var/log/app.log");
        assert_eq!(join_remote("/", "tmp"), "/tmp");
    }

    #[test]
    fn parent_path_handles_relative_and_absolute_paths() {
        assert_eq!(parent_path("var/log"), "var");
        assert_eq!(parent_path("/var/log"), "/var");
        assert_eq!(parent_path("file.txt"), ".");
    }
}
