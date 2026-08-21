use std::path::PathBuf;

use russh_sftp::{client::SftpSession, protocol::OpenFlags};
use tokio::{
    fs::File,
    io::{self, AsyncWriteExt},
};

use crate::error::{AppError, AppResult};

pub enum Operation {
    List { path: String },
    Get { remote: String, local: PathBuf },
    Put { local: PathBuf, remote: String },
    RemoveFile { path: String },
    RemoveDir { path: String },
    MakeDir { path: String },
    PrintWorkingDirectory,
}

pub async fn execute(session: SftpSession, operation: Operation) -> AppResult<()> {
    match operation {
        Operation::List { path } => list(&session, &path).await,
        Operation::Get { remote, local } => download(&session, &remote, &local).await,
        Operation::Put { local, remote } => upload(&session, &local, &remote).await,
        Operation::RemoveFile { path } => session
            .remove_file(path)
            .await
            .map_err(|error| AppError::Sftp(error.to_string())),
        Operation::RemoveDir { path } => session
            .remove_dir(path)
            .await
            .map_err(|error| AppError::Sftp(error.to_string())),
        Operation::MakeDir { path } => session
            .create_dir(path)
            .await
            .map_err(|error| AppError::Sftp(error.to_string())),
        Operation::PrintWorkingDirectory => {
            let path = session
                .canonicalize(".")
                .await
                .map_err(|error| AppError::Sftp(error.to_string()))?;
            println!("{path}");
            Ok(())
        }
    }
}

async fn list(session: &SftpSession, path: &str) -> AppResult<()> {
    let entries = session
        .read_dir(path)
        .await
        .map_err(|error| AppError::Sftp(error.to_string()))?;
    for entry in entries {
        let kind = format!("{:?}", entry.file_type());
        println!(
            "{kind}\t{}\t{}",
            entry.metadata().size.unwrap_or(0),
            entry.file_name()
        );
    }
    Ok(())
}

async fn download(session: &SftpSession, remote: &str, local: &PathBuf) -> AppResult<()> {
    let mut source = session
        .open(remote)
        .await
        .map_err(|error| AppError::Sftp(error.to_string()))?;
    let mut destination = File::create(local).await?;
    io::copy(&mut source, &mut destination).await?;
    destination.flush().await?;
    println!("Downloaded {remote} -> {}", local.display());
    Ok(())
}

async fn upload(session: &SftpSession, local: &PathBuf, remote: &str) -> AppResult<()> {
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
    println!("Uploaded {} -> {remote}", local.display());
    Ok(())
}
