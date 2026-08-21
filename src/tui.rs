use std::time::Duration;
use std::{
    fs,
    io::{self, stdout, Write},
    path::Path,
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};

use crate::{
    app::{Action, App, ProfileDraft},
    error::AppResult,
    profiles::Authentication,
};

const BACKGROUND: Color = Color::Rgb(12, 15, 22);
const SURFACE: Color = Color::Rgb(20, 25, 35);
const SURFACE_ALT: Color = Color::Rgb(27, 34, 48);
const BORDER: Color = Color::Rgb(60, 72, 94);
const MUTED: Color = Color::Rgb(139, 151, 172);
const ACCENT: Color = Color::Rgb(93, 211, 255);
const ACTIVE: Color = Color::Rgb(52, 92, 133);

struct CreateForm {
    values: Vec<String>,
    key_options: Vec<String>,
    field: usize,
    error: String,
}

impl Default for CreateForm {
    fn default() -> Self {
        Self {
            values: vec![
                String::new(),
                String::new(),
                "22".into(),
                String::new(),
                String::new(),
                "password".into(),
                "no".into(),
            ],
            key_options: available_identity_files(),
            field: 0,
            error: String::new(),
        }
    }
}

fn available_identity_files() -> Vec<String> {
    let Some(home) = directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf()) else {
        return Vec::new();
    };
    let ssh_dir = home.join(".ssh");
    let mut keys = fs::read_dir(ssh_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            let is_candidate = path.is_file() && name.starts_with("id_") && !name.ends_with(".pub");
            is_candidate.then(|| path.to_string_lossy().into_owned())
        })
        .collect::<Vec<_>>();
    keys.sort();
    keys
}

pub fn run(app: &mut App) -> AppResult<Option<Action>> {
    enable_raw_mode()?;
    let mut output = stdout();
    execute!(output, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(output);
    let mut terminal = Terminal::new(backend)?;
    let result = event_loop(&mut terminal, app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

pub async fn run_shell(mut channel: russh::Channel<russh::client::Msg>) -> AppResult<()> {
    enable_raw_mode()?;
    let mut output = stdout();
    execute!(output, EnterAlternateScreen)?;
    let result = shell_passthrough(&mut output, &mut channel).await;
    disable_raw_mode()?;
    execute!(output, LeaveAlternateScreen)?;
    output.flush()?;
    result
}

async fn shell_passthrough(
    output: &mut io::Stdout,
    channel: &mut russh::Channel<russh::client::Msg>,
) -> AppResult<()> {
    let mut ticker = tokio::time::interval(Duration::from_millis(15));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            message = channel.wait() => match message {
                Some(russh::ChannelMsg::Data { data })
                | Some(russh::ChannelMsg::ExtendedData { data, .. }) => {
                    output.write_all(&data)?;
                    output.flush()?;
                }
                Some(russh::ChannelMsg::Eof) | Some(russh::ChannelMsg::Close) | None => break,
                _ => {}
            },
            _ = ticker.tick() => {
                while event::poll(Duration::from_millis(0))? {
                    match event::read()? {
                        Event::Key(key) if key.kind == KeyEventKind::Press => {
                            if key.modifiers.contains(KeyModifiers::CONTROL)
                                && matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q'))
                            {
                                return Ok(());
                            }
                            if let Some(bytes) = key_bytes(key) {
                                channel.data(bytes.as_slice()).await?;
                            }
                        }
                        Event::Resize(columns, rows) => {
                            channel
                                .window_change(u32::from(columns), u32::from(rows), 0, 0)
                                .await?;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

fn key_bytes(key: KeyEvent) -> Option<Vec<u8>> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char(character) = key.code {
            let byte = (character.to_ascii_lowercase() as u8).wrapping_sub(b'a' - 1);
            return Some(vec![byte]);
        }
    }
    Some(match key.code {
        KeyCode::Char(character) => character.to_string().into_bytes(),
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Left => b"\x1b[D".to_vec(),
        KeyCode::Right => b"\x1b[C".to_vec(),
        KeyCode::Up => b"\x1b[A".to_vec(),
        KeyCode::Down => b"\x1b[B".to_vec(),
        KeyCode::Home => b"\x1b[H".to_vec(),
        KeyCode::End => b"\x1b[F".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),
        KeyCode::Esc => vec![0x1b],
        _ => return None,
    })
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> AppResult<Option<Action>> {
    let mut create_form = None;
    while !app.should_quit {
        terminal.draw(|frame| {
            draw(frame, app);
            if let Some(form) = create_form.as_ref() {
                draw_create_form(frame, form);
            }
        })?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if let Some(form) = create_form.as_mut() {
                    if let Some(action) = handle_create_key(form, key) {
                        return Ok(Some(action));
                    }
                    if key.code == KeyCode::Esc {
                        create_form = None;
                    }
                } else if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('n') {
                    create_form = Some(CreateForm::default());
                } else if let Some(action) = handle_key(app, key) {
                    return Ok(Some(action));
                }
            }
        }
    }
    Ok(None)
}

fn handle_create_key(form: &mut CreateForm, key: KeyEvent) -> Option<Action> {
    if key.kind != KeyEventKind::Press {
        return None;
    }
    match key.code {
        KeyCode::Tab | KeyCode::Down => {
            form.field = (form.field + 1) % form.values.len();
        }
        KeyCode::BackTab | KeyCode::Up => {
            form.field = form.field.checked_sub(1).unwrap_or(form.values.len() - 1);
        }
        KeyCode::Char(' ') if form.field == 5 => {
            form.values[5] = match form.values[5].as_str() {
                "password" => "private-key".into(),
                "private-key" => "none".into(),
                _ => "password".into(),
            };
        }
        KeyCode::Char(' ') if form.field == 4 => {
            if form.key_options.is_empty() {
                form.error = "No SSH keys found in ~/.ssh.".into();
            } else {
                let next = form
                    .key_options
                    .iter()
                    .position(|key| key == &form.values[4])
                    .map(|index| (index + 1) % form.key_options.len())
                    .unwrap_or(0);
                form.values[4] = form.key_options[next].clone();
                form.error.clear();
            }
        }
        KeyCode::Char(' ') if form.field == 6 => {
            form.values[6] = if form.values[6] == "yes" {
                "no".into()
            } else {
                "yes".into()
            };
        }
        KeyCode::Backspace if form.field != 5 && form.field != 6 => {
            form.values[form.field].pop();
        }
        KeyCode::Char(character) if form.field != 5 && form.field != 6 => {
            form.values[form.field].push(character);
        }
        KeyCode::Enter => return submit_create_form(form),
        _ => {}
    }
    None
}

fn submit_create_form(form: &mut CreateForm) -> Option<Action> {
    if form.values[0].is_empty() || form.values[1].is_empty() || form.values[3].is_empty() {
        form.error = "Name, host and user are required.".into();
        return None;
    }
    if form.values[2].parse::<u16>().is_err() {
        form.error = "Port must be a number between 1 and 65535.".into();
        return None;
    }
    let authentication = match form.values[5].as_str() {
        "private-key" => Authentication::PrivateKey,
        "none" => Authentication::None,
        _ => Authentication::Password,
    };
    if matches!(authentication, Authentication::PrivateKey) && form.values[4].is_empty() {
        form.error = "Identity file is required for private-key auth.".into();
        return None;
    }
    if matches!(authentication, Authentication::PrivateKey) && !Path::new(&form.values[4]).is_file()
    {
        form.error = "Selected identity file does not exist.".into();
        return None;
    }
    Some(Action::Create(ProfileDraft {
        name: form.values[0].clone(),
        host: form.values[1].clone(),
        port: form.values[2].clone(),
        username: form.values[3].clone(),
        identity_file: form.values[4].clone(),
        authentication,
        accept_unknown_host_key: form.values[6] == "yes",
    }))
}

fn handle_key(app: &mut App, key: KeyEvent) -> Option<Action> {
    if key.kind != KeyEventKind::Press {
        return None;
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.quit(),
        KeyCode::Down | KeyCode::Char('j') => app.select_next(),
        KeyCode::Up | KeyCode::Char('k') => app.select_previous(),
        KeyCode::Char('n') => {
            app.status = "Use: sshcli profile add <name> to create a profile.".into();
        }
        KeyCode::Enter => {
            if let Some(profile) = app.selected_profile() {
                return Some(Action::Connect(profile));
            }
            app.status = "No profile selected.".into();
        }
        KeyCode::Char('s') => {
            if let Some(profile) = app.selected_profile() {
                return Some(Action::Sftp(profile));
            }
            app.status = "No profile selected.".into();
        }
        KeyCode::Char('f') => {
            if let Some(profile) = app.selected_profile() {
                return Some(Action::Forward(profile));
            }
            app.status = "No profile selected.".into();
        }
        KeyCode::Char('d') => {
            if let Some(profile) = app.selected_profile() {
                if app.delete_confirmation.as_deref() == Some(profile.name.as_str()) {
                    return Some(Action::Delete(profile));
                }
                app.delete_confirmation = Some(profile.name.clone());
                app.status = format!("Press d again to delete '{}'.", profile.name);
            } else {
                app.status = "No profile selected.".into();
            }
        }
        _ => {}
    }
    None
}

fn draw(frame: &mut Frame, app: &App) {
    frame.render_widget(
        Block::default().style(Style::default().bg(BACKGROUND)),
        frame.area(),
    );
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(frame.area());
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " SSHCLI ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled("/ CONNECTIONS", Style::default().fg(MUTED)),
        Span::raw("                                      "),
        Span::styled(
            format!("{} saved", app.profiles.len()),
            Style::default().fg(MUTED),
        ),
    ]))
    .style(Style::default().bg(SURFACE))
    .block(panel(""));
    frame.render_widget(header, outer[0]);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(34), Constraint::Min(0)])
        .split(outer[1]);

    let items = app
        .profiles
        .iter()
        .map(|profile| {
            ListItem::new(vec![
                Line::from(Span::styled(
                    format!("  {}", profile.name),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("  {}@{}:{}", profile.username, profile.host, profile.port),
                    Style::default().fg(MUTED),
                )),
            ])
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    state.select((!app.profiles.is_empty()).then_some(app.selected_profile));
    let profile_list = List::new(items)
        .style(Style::default().bg(SURFACE))
        .block(panel(" Connections "))
        .highlight_style(Style::default().bg(ACTIVE).fg(Color::White))
        .highlight_symbol(" ");
    frame.render_stateful_widget(profile_list, columns[0], &mut state);

    let detail_lines = if let Some(profile) = app.profiles.get(app.selected_profile) {
        vec![
            Line::from(Span::styled(
                &profile.name,
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled("READY", Style::default().fg(Color::Green))),
            Line::from(""),
            Line::from(Span::styled("Endpoint", Style::default().fg(MUTED))),
            Line::from(format!("{}:{}", profile.host, profile.port)),
            Line::from(Span::styled("User", Style::default().fg(MUTED))),
            Line::from(profile.username.as_str()),
            Line::from(""),
            Line::from(Span::styled("Enter", Style::default().fg(Color::Yellow))),
            Line::from("open SSH session"),
            Line::from(Span::styled("s", Style::default().fg(Color::Yellow))),
            Line::from("browse files with SFTP"),
            Line::from(Span::styled("f", Style::default().fg(Color::Yellow))),
            Line::from("start local forwarding"),
            Line::from(Span::styled("d", Style::default().fg(Color::Yellow))),
            Line::from("delete this connection"),
        ]
    } else {
        vec![
            Line::from(Span::styled(
                "No connections yet",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )),
            Line::from("Press n to create your first SSH profile."),
            Line::from(""),
            Line::from(Span::styled("Your workspace", Style::default().fg(MUTED))),
            Line::from("SSH sessions, SFTP browsers and tunnels"),
            Line::from("will appear here when you connect."),
        ]
    };
    let detail = Paragraph::new(detail_lines)
        .style(Style::default().fg(Color::White).bg(SURFACE))
        .block(panel(" Workspace "));
    frame.render_widget(detail, columns[1]);

    let footer = Paragraph::new(Line::from(vec![
        key_hint("q", "quit"),
        Span::raw("  "),
        key_hint("n", "new"),
        Span::raw("  "),
        key_hint("Enter", "connect"),
        Span::raw("  "),
        key_hint("s", "sftp"),
        Span::raw("  "),
        key_hint("f", "forward"),
        Span::raw("  "),
        key_hint("d", "delete"),
        Span::raw("   "),
        Span::styled(app.status.as_str(), Style::default().fg(MUTED)),
    ]))
    .style(Style::default().bg(SURFACE_ALT))
    .block(panel(""));
    frame.render_widget(footer, outer[2]);
}

fn panel(title: &str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .title(title.to_string())
}

fn key_hint(key: &str, label: &str) -> Span<'static> {
    Span::styled(format!(" {key} {label}"), Style::default().fg(MUTED))
}

fn draw_create_form(frame: &mut Frame, form: &CreateForm) {
    let area = centered_rect(70, 70, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" New Connection ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let labels = [
        "Name",
        "Host",
        "Port",
        "User",
        "Identity file",
        "Authentication",
        "Accept unknown host key",
    ];
    let lines = labels
        .iter()
        .enumerate()
        .map(|(index, label)| {
            let style = if index == form.field {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Line::from(vec![
                Span::styled(format!("{label:<24}"), style),
                Span::styled(form.values[index].as_str(), style),
            ])
        })
        .collect::<Vec<_>>();
    let help = if form.error.is_empty() {
        if form.field == 4 {
            "Space: select existing ~/.ssh key | Enter: save | Esc: cancel"
        } else {
            "Tab/Up/Down: field | Space: select | Enter: save | Esc: cancel"
        }
    } else {
        form.error.as_str()
    };
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(inner);
    frame.render_widget(Paragraph::new(lines), sections[0]);
    frame.render_widget(
        Paragraph::new(help).style(if form.error.is_empty() {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::Red)
        }),
        sections[1],
    );
}

fn centered_rect(horizontal: u16, vertical: u16, area: Rect) -> Rect {
    let vertical_margin = (100 - vertical) / 2;
    let horizontal_margin = (100 - horizontal) / 2;
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(vertical_margin),
            Constraint::Percentage(vertical),
            Constraint::Percentage(vertical_margin),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(horizontal_margin),
            Constraint::Percentage(horizontal),
            Constraint::Percentage(horizontal_margin),
        ])
        .split(rows[1])[1]
}
