use std::io::{self, stdout};
use std::time::Duration;

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
    app::{Action, App},
    error::AppResult,
};

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
    while !app.should_quit {
        terminal.draw(|frame| draw(frame, app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if let Some(action) = handle_key(app, key) {
                    return Ok(Some(action));
                }
            }
        }
    }
    Ok(None)
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
        Span::raw(app.status.as_str()),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, outer[1]);
}
