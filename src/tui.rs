use std::time::Duration;
use std::{
    fs,
    io::{self, stdout},
    path::Path,
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};

use crate::{
    app::{Action, App, ProfileDraft},
    error::AppResult,
    profiles::Authentication,
};

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

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> AppResult<Option<Action>> {
    let mut create_form = None;
    while !app.should_quit {
        terminal.draw(|frame| {
            if let Some(form) = create_form.as_ref() {
                draw_create_form(frame, form);
            } else {
                draw(frame, app);
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
        _ => {}
    }
    None
}

fn draw(frame: &mut Frame, app: &App) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(frame.area());
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(30), Constraint::Min(0)])
        .split(outer[0]);

    let items = app
        .profiles
        .iter()
        .map(|profile| ListItem::new(profile.name.as_str()))
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    state.select(Some(app.selected_profile));
    let profile_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Connections "),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    frame.render_stateful_widget(profile_list, columns[0], &mut state);

    let detail = Paragraph::new(vec![
        Line::from(Span::styled(
            "SSHCLI",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("A cross-platform SSH client for the terminal."),
        Line::from(""),
        Line::from("Phase 1: TUI foundation"),
        Line::from("Phase 2: SSH interactive sessions available via connect"),
        Line::from("Phase 3: profiles and secure credentials available"),
    ])
    .block(Block::default().borders(Borders::ALL).title(" Workspace "));
    frame.render_widget(detail, columns[1]);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" q ", Style::default().fg(Color::Yellow)),
        Span::raw("quit  "),
        Span::styled(" n ", Style::default().fg(Color::Yellow)),
        Span::raw("new profile  "),
        Span::styled(" Enter ", Style::default().fg(Color::Yellow)),
        Span::raw("connect  |  "),
        Span::styled(" s ", Style::default().fg(Color::Yellow)),
        Span::raw("sftp  |  "),
        Span::styled(" f ", Style::default().fg(Color::Yellow)),
        Span::raw("forward  |  "),
        Span::raw(app.status.as_str()),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, outer[1]);
}

fn draw_create_form(frame: &mut Frame, form: &CreateForm) {
    let area = frame.area();
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
    let content = Paragraph::new(lines).block(Block::default().title(help));
    frame.render_widget(content, inner);
}
