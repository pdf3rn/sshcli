use std::path::PathBuf;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
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

struct RemoteEntry {
    name: String,
    is_dir: bool,
    size: u64,
}

pub async fn browse(session: SftpSession) -> AppResult<()> {
    enable_raw_mode()?;
    let mut output = std::io::stdout();
    execute!(output, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(output);
    let mut terminal = Terminal::new(backend)?;
    let result = browse_loop(&mut terminal, session).await;
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

async fn browse_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    session: SftpSession,
) -> AppResult<()> {
    let mut path = ".".to_string();
    let mut entries = load_entries(&session, &path).await?;
    let mut selected = 0_usize;
    let mut status = "Enter: open directory | d: download | q: exit".to_string();

    loop {
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(2)])
                .split(frame.area());
            let items = entries
                .iter()
                .map(|entry| {
                    let marker = if entry.is_dir { "[DIR]" } else { "     " };
                    ListItem::new(format!("{marker} {:>10} {}", entry.size, entry.name))
                })
                .collect::<Vec<_>>();
            let mut state = ListState::default();
            state.select((!entries.is_empty()).then_some(selected));
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(path.as_str()))
                .highlight_style(
                    Style::default()
                        .bg(Color::Blue)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, chunks[0], &mut state);
            frame.render_widget(
                Paragraph::new(status.as_str()).block(Block::default().borders(Borders::ALL)),
                chunks[1],
            );
        })?;

        if !event::poll(std::time::Duration::from_millis(250))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
            KeyCode::Down | KeyCode::Char('j') if !entries.is_empty() => {
                selected = (selected + 1) % entries.len();
            }
            KeyCode::Up | KeyCode::Char('k') if !entries.is_empty() => {
                selected = selected.checked_sub(1).unwrap_or(entries.len() - 1);
            }
            KeyCode::Backspace => {
                if path != "/" && path != "." {
                    path = parent_path(&path);
                    entries = load_entries(&session, &path).await?;
                    selected = 0;
                }
            }
            KeyCode::Enter if !entries.is_empty() && entries[selected].is_dir => {
                path = join_remote(&path, &entries[selected].name);
                entries = load_entries(&session, &path).await?;
                selected = 0;
            }
            KeyCode::Char('d') if !entries.is_empty() && !entries[selected].is_dir => {
                let entry = &entries[selected];
                let remote = join_remote(&path, &entry.name);
                let local = std::env::current_dir()?.join(&entry.name);
                match download(&session, &remote, &local).await {
                    Ok(()) => status = format!("Downloaded to {}", local.display()),
                    Err(error) => status = error.to_string(),
                }
            }
            _ => {}
        }
    }
}

async fn load_entries(session: &SftpSession, path: &str) -> AppResult<Vec<RemoteEntry>> {
    let entries = session
        .read_dir(path)
        .await
        .map_err(|error| AppError::Sftp(error.to_string()))?;
    Ok(entries
        .map(|entry| RemoteEntry {
            name: entry.file_name(),
            is_dir: entry.file_type().is_dir(),
            size: entry.metadata().size.unwrap_or(0),
        })
        .collect())
}

fn join_remote(parent: &str, child: &str) -> String {
    if parent == "." {
        child.to_string()
    } else if parent == "/" {
        format!("/{child}")
    } else {
        format!("{parent}/{child}")
    }
}

fn parent_path(path: &str) -> String {
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
