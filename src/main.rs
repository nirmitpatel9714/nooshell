mod app;
mod config;
mod execution;
mod pane;
mod script;
mod state;
mod store;

use crate::app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use std::{env, error::Error, io, io::Write, time::Duration};
use crossterm::event::KeyEventKind;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = config::load_config("languages.json").unwrap_or_default();
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|a| a == "nbmode") {
        let mut app = App::new(config);
        run_tui(&mut app).await?;
    } else if args.iter().any(|a| a == "clearc") {
        store::clear_history();
        println!("Command history cleared.");
    } else if args.iter().any(|a| a == "delses") {
        if let Some(id) = args.iter().position(|a| a == "delses").and_then(|p| args.get(p + 1)) {
            if store::delete_session(id) {
                println!("Session '{}' deleted.", id);
            } else {
                println!("Session '{}' not found.", id);
            }
        } else {
            println!("Usage: noo delses <session_id>");
        }
    } else if args.iter().any(|a| a == "history") {
        let hist = store::load_history();
        if hist.commands.is_empty() {
            println!("No command history.");
        } else {
            for cmd in hist.commands.iter().rev().take(50) {
                println!("#{} [{}] {}  --  {}", cmd.id, cmd.language, cmd.command, cmd.timestamp);
            }
        }
    } else if args.iter().any(|a| a == "sessions") {
        let sessions = store::list_sessions();
        if sessions.is_empty() {
            println!("No saved sessions.");
        } else {
            for s in &sessions {
                println!("{}  |  {}  |  {} workspaces, {} cells total", s.id, s.name, s.workspaces.len(), 
                    s.workspaces.iter().map(|w| w.cells.len()).sum::<usize>());
            }
        }
    } else if args.iter().any(|a| a == "manage") {
        let mut app = App::new(config);
        run_manage_tui(&mut app).await?;
    } else {
        let mut app = App::new(config);
        run_cli(&mut app).await?;
    }

    Ok(())
}

async fn run_cli(app: &mut App) -> Result<(), Box<dyn Error>> {
    println!("Nooshell CLI mode. Type 'exit' to quit. Use 'noo nbmode' to enter Notebook mode.");

    loop {
        app.poll_all_panes();

        {
            let ws = &mut app.workspaces[app.active_workspace];
            for pane in &mut ws.panes {
                if !pane.output_lines.is_empty() {
                    for line in &pane.output_lines {
                        println!("\x1b[36m[{}]\x1b[0m: {}", pane.active_language, line);
                    }
                    pane.output_lines.clear();
                }
            }
        }

        let current_dir = env::current_dir().unwrap_or_else(|_| env::temp_dir());
        let dir_name = current_dir.file_name().unwrap_or_default().to_string_lossy();
        let lang = {
            let ws = &app.workspaces[app.active_workspace];
            ws.panes[ws.active_pane].active_language.clone()
        };

        eprintln!("[nooshell-debug] history path: {}", store::history_path_str());
        let hist = store::load_history();
        eprintln!("[nooshell-debug] history entries on load: {}", hist.commands.len());
        for c in &hist.commands {
            eprintln!("[nooshell-debug]   cmd: {}", c.command);
        }

        let prompt = format!("\x1b[32;1m➜ \x1b[36;1m[{}]\x1b[0m \x1b[33m({})\x1b[0m \x1b[35m❯\x1b[0m ", dir_name, lang);
        let input = readline_with_history("\n", &prompt)?;
        let input = input.trim().to_string();

        if input.is_empty() {
            continue;
        }

        let switch_lang = {
            let ws = &app.workspaces[app.active_workspace];
            let parts: Vec<&str> = input.split_whitespace().collect();
            if parts.len() == 1 {
                ws.panes.iter().position(|p| p.active_language == parts[0])
            } else {
                None
            }
        };
        if let Some(pos) = switch_lang {
            app.workspaces[app.active_workspace].active_pane = pos;
            continue;
        }

        let parts: Vec<&str> = input.split_whitespace().collect();

        if parts.first() == Some(&"clear") {
            print!("\x1b[2J\x1b[1;1H");
        }

        // Handle noo subcommands inside CLI mode
        if parts.first() == Some(&"noo") && parts.len() > 1 {
            match parts[1] {
                "nbmode" => {
                    run_tui(app).await?;
                    break;
                }
                "clearc" => {
                    store::clear_history();
                    println!("\x1b[33mCommand history cleared.\x1b[0m");
                    continue;
                }
                "delses" => {
                    if let Some(id) = parts.get(2) {
                        if store::delete_session(id) {
                            println!("\x1b[33mSession '{}' deleted.\x1b[0m", id);
                        } else {
                            println!("\x1b[31mSession '{}' not found.\x1b[0m", id);
                        }
                    } else {
                        println!("\x1b[31mUsage: noo delses <session_id>\x1b[0m");
                    }
                    continue;
                }
                "history" => {
                    let hist = store::load_history();
                    if hist.commands.is_empty() {
                        println!("  No command history.");
                    } else {
                        for cmd in hist.commands.iter().rev().take(50) {
                            println!("  \x1b[36m[{}]\x1b[0m {}  --  {}", cmd.language, cmd.command, cmd.timestamp);
                        }
                    }
                    continue;
                }
                "sessions" => {
                    let sessions = store::list_sessions();
                    if sessions.is_empty() {
                        println!("  No saved sessions.");
                    } else {
                        for s in &sessions {
                            let cell_total: usize = s.workspaces.iter().map(|w| w.cells.len()).sum();
                            println!("  \x1b[33m{}\x1b[0m  |  {} workspaces, {} cells", s.name, s.workspaces.len(), cell_total);
                        }
                    }
                    continue;
                }
                "manage" => {
                    run_manage_tui(app).await?;
                    continue;
                }
                _ => {}
            }
        }

        app.current_pane_mut().input_buffer = input.clone();
        if let Some(cmd) = app.current_pane_mut().handle_input().await {
            if cmd == "exit" {
                break;
            }
        }

        app.record_command(&lang, &input, &[]);
        let hist = store::load_history();
        eprintln!("[nooshell-debug] history entries after record: {}", hist.commands.len());
    }

    Ok(())
}

fn readline_with_history(prefix: &str, prompt: &str) -> io::Result<String> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();

    let mut input = String::new();
    let history = store::load_history();
    let entries: Vec<String> = history.commands.iter().map(|c| c.command.clone()).collect();
    let mut hist_idx = entries.len();

    write!(stdout, "{}{}", prefix, prompt)?;
    stdout.flush()?;

    loop {
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Enter => {
                            writeln!(stdout)?;
                            stdout.flush()?;
                            break;
                        }
                        KeyCode::Up => {
                            if hist_idx > 0 {
                                hist_idx -= 1;
                                input = entries[hist_idx].clone();
                            }
                        }
                        KeyCode::Down => {
                            if hist_idx + 1 < entries.len() {
                                hist_idx += 1;
                                input = entries[hist_idx].clone();
                            } else {
                                hist_idx = entries.len();
                                input.clear();
                            }
                        }
                        KeyCode::Backspace => {
                            input.pop();
                        }
                        KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' => {
                            writeln!(stdout, "^C")?;
                            stdout.flush()?;
                            disable_raw_mode()?;
                            return Ok(String::new());
                        }
                        KeyCode::Char(c) => {
                            input.push(c);
                        }
                        _ => {}
                    }
                    write!(stdout, "\r\x1b[2K{}{}", prompt, input)?;
                    stdout.flush()?;
                }
            }
        }
    }

    disable_raw_mode()?;
    Ok(input)
}

// ── Notebook TUI ──

async fn run_tui(app: &mut App) -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, app).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> 
where
    std::io::Error: From<<B as Backend>::Error>,
{
    loop {
        app.poll_all_panes();

        terminal.draw(|f| {
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(1)])
                .split(f.area());

            // ── workspace tab bar ──
            {
                let mut tab_spans: Vec<ratatui::text::Span> = Vec::new();
                for (i, ws) in app.workspaces.iter().enumerate() {
                    if i > 0 {
                        tab_spans.push(ratatui::text::Span::raw(" │ "));
                    }
                    let active_ws = i == app.active_workspace;
                    let style = if active_ws {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    tab_spans.push(ratatui::text::Span::styled(
                        format!(" {} ", ws.name),
                        style,
                    ));
                }
                tab_spans.push(ratatui::text::Span::raw("   "));
                tab_spans.push(ratatui::text::Span::styled(
                    "Ctrl+M manage",
                    Style::default().fg(Color::DarkGray),
                ));
                let tab_line = ratatui::text::Line::from(tab_spans);
                f.render_widget(Paragraph::new(tab_line), main_chunks[0]);
            }

            // ── notebook cells for active workspace ──
            let ws = &app.workspaces[app.active_workspace];
            let cell_count = ws.panes.len();
            let active_idx = ws.active_pane;

            let mut constraints: Vec<Constraint> = Vec::new();
            for i in 0..cell_count {
                if i == active_idx {
                    constraints.push(Constraint::Min(5));
                } else {
                    constraints.push(Constraint::Length(3));
                }
            }
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(main_chunks[1]);

            for (i, pane) in ws.panes.iter().enumerate() {
                let active = i == active_idx;
                let title = format!("[{}/{}] Cell {} ({}) ", i + 1, cell_count, i + 1, pane.active_language);
                let border_style = if active {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                let block = Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style);

                if active {
                    let mut lines: Vec<ratatui::text::Line> = Vec::new();
                    for out_line in &pane.output_lines {
                        let styled = if out_line.starts_with("In [") {
                            ratatui::text::Line::from(vec![
                                ratatui::text::Span::styled(out_line, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                            ])
                        } else if out_line.starts_with("Out[") || out_line.starts_with("    ") {
                            ratatui::text::Line::from(vec![
                                ratatui::text::Span::styled(out_line, Style::default().fg(Color::Cyan))
                            ])
                        } else {
                            ratatui::text::Line::from(out_line.clone())
                        };
                        lines.push(styled);
                    }

                    lines.push(ratatui::text::Line::from(""));
                    let prompt_line = ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled(
                            format!("In [{}]: ", pane.execution_count + 1),
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                        ),
                        ratatui::text::Span::raw(&pane.input_buffer),
                        ratatui::text::Span::styled("█", Style::default().fg(Color::Yellow)),
                    ]);
                    lines.push(prompt_line);

                    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
                    f.render_widget(paragraph, chunks[i]);
                } else {
                    let paragraph = Paragraph::new(vec![
                        ratatui::text::Line::from(format!(" {} outputs · {} history", pane.output_lines.len(), pane.history.len()))
                    ]).block(block);
                    f.render_widget(paragraph, chunks[i]);
                }
            }
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc => {
                            app.running = false;
                            return Ok(());
                        }
                        KeyCode::Left => {
                            app.previous_workspace();
                        }
                        KeyCode::Right => {
                            app.next_workspace();
                        }
                        KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                            app.move_cell_up();
                        }
                        KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                            app.move_cell_down();
                        }
                        KeyCode::Up if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            let pane = app.current_pane_mut();
                            if pane.history_index > 0 {
                                pane.history_index -= 1;
                                pane.input_buffer = pane.history[pane.history_index].clone();
                            }
                        }
                        KeyCode::Down if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            let pane = app.current_pane_mut();
                            if pane.history_index + 1 < pane.history.len() {
                                pane.history_index += 1;
                                pane.input_buffer = pane.history[pane.history_index].clone();
                            } else {
                                pane.history_index = pane.history.len();
                                pane.input_buffer.clear();
                            }
                        }
                        KeyCode::Up => {
                            let ws = app.current_workspace_mut();
                            if ws.active_pane > 0 {
                                ws.active_pane -= 1;
                            }
                        }
                        KeyCode::Down => {
                            let ws = app.current_workspace_mut();
                            if ws.active_pane + 1 < ws.panes.len() {
                                ws.active_pane += 1;
                            }
                        }
                        KeyCode::Tab => {
                            let ws = app.current_workspace_mut();
                            if ws.active_pane + 1 < ws.panes.len() {
                                ws.active_pane += 1;
                            }
                        }
                        KeyCode::BackTab => {
                            let ws = app.current_workspace_mut();
                            if ws.active_pane > 0 {
                                ws.active_pane -= 1;
                            }
                        }
                        KeyCode::Enter => {
                            let lang = {
                                let p = app.current_pane_mut();
                                p.active_language.clone()
                            };
                            let input = {
                                let p = app.current_pane_mut();
                                p.input_buffer.clone()
                            };
                            if let Some(cmd) = app.current_pane_mut().handle_input().await {
                                if cmd == "exit" {
                                    app.running = false;
                                    return Ok(());
                                }
                            }
                            let output = {
                                let p = app.current_pane_mut();
                                p.output_lines.clone()
                            };
                            app.record_command(&lang, &input, &output);
                        }
                        KeyCode::Backspace => {
                            app.current_pane_mut().input_buffer.pop();
                        }
                        KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::CONTROL) && key.modifiers.contains(KeyModifiers::SHIFT) => match c {
                            'W' => app.remove_workspace(),
                            _ => {}
                        },
                        KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::CONTROL) => match c {
                            't' => app.add_cell(),
                            'w' => app.remove_cell(),
                            'n' => app.add_workspace(),
                            'm' => {
                                let _ = run_manage_tui(app).await;
                                enable_raw_mode()?;
                                execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
                            }
                            _ => {}
                        },
                        KeyCode::Char(c) => {
                            app.current_pane_mut().input_buffer.push(c);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

// ── Management TUI ──

#[derive(PartialEq)]
enum ManageTab {
    Sessions,
    History,
}

async fn run_manage_tui(_app: &mut App) -> Result<(), Box<dyn Error>> {
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut tab = ManageTab::Sessions;
    let mut list_index = 0usize;

    let res: io::Result<()> = loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(1)])
                .split(f.area());

            // tabs
            let sess_style = if tab == ManageTab::Sessions {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let hist_style = if tab == ManageTab::History {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let tabs = Paragraph::new(vec![
                ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled(" Sessions ", sess_style),
                    ratatui::text::Span::raw(" │ "),
                    ratatui::text::Span::styled(" History ", hist_style),
                ]),
                ratatui::text::Line::from(
                    ratatui::text::Span::styled(
                        " Tab/Shift+Tab: switch · d: delete · Esc: back",
                        Style::default().fg(Color::DarkGray),
                    )
                ),
            ]);
            f.render_widget(tabs, chunks[0]);

            // list
            let list_block = Block::default()
                .borders(Borders::ALL)
                .title(if tab == ManageTab::Sessions { " Saved Sessions " } else { " Command History " });
            let mut list_lines: Vec<ratatui::text::Line> = Vec::new();

            match tab {
                ManageTab::Sessions => {
                    let sessions = store::list_sessions();
                    if sessions.is_empty() {
                        list_lines.push(ratatui::text::Line::from("  No saved sessions."));
                    } else {
                        for (i, s) in sessions.iter().enumerate() {
                            let prefix = if i == list_index { "▸ " } else { "  " };
                            let style = if i == list_index {
                                Style::default().fg(Color::Yellow)
                            } else {
                                Style::default()
                            };
                            let cell_total: usize = s.workspaces.iter().map(|w| w.cells.len()).sum();
                            let text = format!("{}{}  {} workspaces, {} cells", prefix, s.name, s.workspaces.len(), cell_total);
                            list_lines.push(ratatui::text::Line::from(
                                ratatui::text::Span::styled(text, style)
                            ));
                        }
                    }
                }
                ManageTab::History => {
                    let hist = store::load_history();
                    if hist.commands.is_empty() {
                        list_lines.push(ratatui::text::Line::from("  No command history."));
                    } else {
                        let start = list_index.min(hist.commands.len().saturating_sub(1));
                        for (i, cmd) in hist.commands.iter().rev().skip(start).take(20).enumerate() {
                            let idx = start + i;
                            let prefix = if idx == list_index { "▸ " } else { "  " };
                            let style = if idx == list_index {
                                Style::default().fg(Color::Yellow)
                            } else {
                                Style::default()
                            };
                            list_lines.push(ratatui::text::Line::from(
                                ratatui::text::Span::styled(
                                    format!("{}{} [{}]  {}", prefix, cmd.command, cmd.language, cmd.timestamp),
                                    style,
                                )
                            ));
                        }
                    }
                }
            }

            f.render_widget(Paragraph::new(list_lines).block(list_block), chunks[1]);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc => break Ok(()),
                        KeyCode::Tab => {
                            tab = if tab == ManageTab::Sessions { ManageTab::History } else { ManageTab::Sessions };
                            list_index = 0;
                        }
                        KeyCode::BackTab => {
                            tab = if tab == ManageTab::History { ManageTab::Sessions } else { ManageTab::History };
                            list_index = 0;
                        }
                        KeyCode::Up => {
                            list_index = list_index.saturating_sub(1);
                        }
                        KeyCode::Down => {
                            list_index += 1;
                        }
                        KeyCode::Char('d') => {
                            match tab {
                                ManageTab::Sessions => {
                                    let sessions = store::list_sessions();
                                    if list_index < sessions.len() {
                                        store::delete_session(&sessions[list_index].id);
                                        list_index = list_index.min(sessions.len().saturating_sub(2));
                                    }
                                }
                                ManageTab::History => {
                                    store::clear_history();
                                    list_index = 0;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    };

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res.map_err(|e| e.into())
}
