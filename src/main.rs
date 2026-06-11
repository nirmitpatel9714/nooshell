//! # nooshell binary
//!
//! CLI entry point. Parses arguments and dispatches to:
//! - **CLI mode** (`noo`) — single-pane REPL with history
//! - **Notebook TUI** (`noo nbmode`) — multi-workspace tabbed notebook
//! - **Management TUI** (`noo manage`) — session & history management
//! - **Script mode** (`noo <script.ns>`) — batch script execution
//! - **Compile mode** (`noo compile <script.ns>`) — compile to native binary

use nooshell::app::App;
use nooshell::config;
use nooshell::noorc;
use nooshell::passthrough;
use nooshell::script::NsScript;
use nooshell::shell_resolver;
use nooshell::store;
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
use std::{env, error::Error, io, io::Write, path::Path, time::Duration};
use crossterm::event::KeyEventKind;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    // noo compile <script.ns> [--windows | --linux | --mac]
    if args.iter().any(|a| a == "compile") {
        if let Some(pos) = args.iter().position(|a| a == "compile") {
            if let Some(script_path) = args.get(pos + 1) {
                let path = Path::new(script_path);
                if !path.exists() {
                    eprintln!("Script not found: {}", script_path);
                    std::process::exit(1);
                }
                use nooshell::compile::Target;
                let target = if args.contains(&"--windows".to_string()) {
                    Target::Windows
                } else if args.contains(&"--linux".to_string()) {
                    Target::Linux
                } else if args.contains(&"--mac".to_string()) {
                    Target::Mac
                } else {
                    Target::Native
                };
                nooshell::compile::compile(path, target)?;
                return Ok(());
            } else {
                eprintln!("Usage: noo compile <script.ns> [--windows | --linux | --mac]");
                std::process::exit(1);
            }
        }
    }

    let lang_json_embedded = include_str!("../config/languages.json");
    let config = config::load_language_config(lang_json_embedded);
    let noorc = noorc::Noorc::load();
    let startup = noorc.startup.clone();

    if let Some(script_path) = args.iter().find(|a| a.ends_with(".ns")) {
        let content = match std::fs::read_to_string(script_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to read script: {}", e);
                std::process::exit(1);
            }
        };
        NsScript::run_embedded(&content, lang_json_embedded).await;
        return Ok(());
    }

    if args.iter().any(|a| a == "nbmode") {
        let mut app = App::with_noorc(config, noorc.language.as_deref(), noorc.aliases.clone());

        let autosave_exists = store::list_sessions().iter().any(|s| s.id == "_autosave");
        let mut restored = false;
        if autosave_exists {
            print!("Autosaved session found. Restore? [Y/n]: ");
            io::stdout().flush()?;
            let mut answer = String::new();
            io::stdin().read_line(&mut answer)?;
            if !answer.trim().eq_ignore_ascii_case("n") {
                restored = app.restore_from_autosave();
                if restored {
                    println!("Session restored.");
                } else {
                    println!("Could not restore session.");
                }
            } else {
                println!("Starting fresh session.");
            }
        }

        if !restored {
            for cmd in &startup {
                app.current_pane_mut().input_buffer = cmd.clone();
                app.current_pane_mut().handle_input().await;
            }
        }
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
    } else if let Some(pos) = args.iter().position(|a| a == "pass") {
        let shell_name = args.get(pos + 1).cloned().unwrap_or_default();
        if shell_name.is_empty() {
            let supported = shell_resolver::list_supported_shells().join(", ");
            eprintln!("Usage: noo pass <shell>");
            eprintln!("Supported shells: {}", supported);
            std::process::exit(1);
        }
        let cfg = passthrough::PassThroughConfig {
            shell_name,
            ..Default::default()
        };
        if let Err(e) = passthrough::run_passthrough(&cfg) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    } else {
        let mut app = App::with_noorc(config, noorc.language.as_deref(), noorc.aliases);
        run_cli(&mut app, &startup).await?;
    }

    Ok(())
}

fn parse_lang_command(input: &str, config: &config::ConfigMap) -> Option<(String, String)> {
    let trimmed = input.trim();
    // lang(code) syntax
    if let Some(close) = trimmed.find(')') {
        if let Some(open) = trimmed.find('(') {
            if close > open + 1 {
                let lang = trimmed[..open].trim();
                let code = &trimmed[open + 1..close];
                if config.contains_key(lang) && !code.is_empty() {
                    return Some((lang.to_string(), code.to_string()));
                }
            }
        }
    }
    // lang code syntax
    let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
    if parts.len() == 2 && config.contains_key(parts[0]) {
        return Some((parts[0].to_string(), parts[1].to_string()));
    }
    None
}

async fn run_cli(app: &mut App, startup: &[String]) -> Result<(), Box<dyn Error>> {
    for cmd in startup {
        app.current_pane_mut().input_buffer = cmd.clone();
        app.current_pane_mut().handle_input().await;
    }

    loop {
        let current_dir = env::current_dir().unwrap_or_else(|_| env::temp_dir());
        let dir_name = current_dir.file_name().unwrap_or_default().to_string_lossy();
        let lang = {
            let ws = &app.workspaces[app.active_workspace];
            ws.panes[ws.active_pane].active_language.clone()
        };

        let prompt = format!("\x1b[32;1m➜ \x1b[36;1m[{}]\x1b[0m \x1b[33m({})\x1b[0m \x1b[35m❯\x1b[0m ", dir_name, lang);
        let input = readline_with_history("\n", &prompt, app)?;
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
                    app.record_command(&lang, &input, &[]);
                    run_tui(app).await?;
                    break;
                }
                "clearc" => {
                    app.record_command(&lang, &input, &[]);
                    store::clear_history();
                    println!("\x1b[33mCommand history cleared.\x1b[0m");
                    continue;
                }
                "delses" => {
                    app.record_command(&lang, &input, &[]);
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
                "pass" => {
                    let shell_name = parts.get(2).map(|s| s.to_string()).unwrap_or_default();
                    if shell_name.is_empty() {
                        let supported = shell_resolver::list_supported_shells().join(", ");
                        println!("\x1b[33mUsage: noo pass <shell>\x1b[0m");
                        println!("\x1b[33mSupported shells: {}\x1b[0m", supported);
                        continue;
                    }
                    app.record_command(&lang, &input, &[]);
                    let cfg = passthrough::PassThroughConfig {
                        shell_name,
                        ..Default::default()
                    };
                    match passthrough::run_passthrough(&cfg) {
                        Ok(()) => {}
                        Err(e) => {
                            eprintln!("\x1b[31mError: {}\x1b[0m", e);
                        }
                    }
                    continue;
                }
                "manage" => {
                    app.record_command(&lang, &input, &[]);
                    run_manage_tui(app).await?;
                    continue;
                }
                lang if app.config.contains_key(lang) => {
                    app.record_command(&lang, &input, &[]);
                    let state = app.state.clone();
                    let ws = &mut app.workspaces[app.active_workspace];
                    let idx = ws.ensure_pane(lang, &app.config, state);
                    ws.active_pane = idx;
                    continue;
                }
                _ => {}
            }
        }

        if let Some((target_lang, cmd)) = parse_lang_command(&input, &app.config) {
            let state = app.state.clone();
            let ws = &mut app.workspaces[app.active_workspace];
            let idx = ws.ensure_pane(&target_lang, &app.config, state);
            ws.active_pane = idx;
            app.current_pane_mut().input_buffer = cmd.clone();
            app.current_pane_mut().handle_input().await;
            app.record_command(&target_lang, &cmd, &[]);
            app.auto_save();
            continue;
        }

        app.current_pane_mut().input_buffer = input.clone();
        if let Some(cmd) = app.current_pane_mut().handle_input().await {
            if cmd == "exit" {
                app.auto_save();
                break;
            }
        }

        app.record_command(&lang, &input, &[]);
        app.auto_save();
    }

    Ok(())
}

fn readline_with_history(prefix: &str, prompt: &str, app: &mut App) -> io::Result<String> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();

    let mut input = String::new();
    let mut cursor_pos: usize = 0;
    let history = store::load_history();
    let entries: Vec<String> = history.commands.iter().map(|c| c.command.clone()).collect();
    let mut hist_idx = entries.len();

    write!(stdout, "{}\r{}", prefix, prompt)?;
    stdout.flush()?;

    loop {
        app.poll_all_panes();

        let output_lines: Vec<String> = {
            let ws = &mut app.workspaces[app.active_workspace];
            let mut lines = Vec::new();
            for pane in &mut ws.panes {
                lines.append(&mut pane.output_lines);
            }
            lines
        };

        if !output_lines.is_empty() {
            write!(stdout, "\r\x1b[2K")?;
            for line in &output_lines {
                writeln!(stdout, "{}", line)?;
            }
            let (before, after) = input.split_at(cursor_pos);
            write!(stdout, "{}{}{}", prompt, before, after)?;
            if after.len() > 0 {
                write!(stdout, "\x1b[{}D", after.len())?;
            }
            stdout.flush()?;
        }

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Enter => {
                            writeln!(stdout)?;
                            stdout.flush()?;
                            break;
                        }
                        KeyCode::Left => {
                            if cursor_pos > 0 {
                                cursor_pos -= 1;
                            }
                        }
                        KeyCode::Right => {
                            if cursor_pos < input.len() {
                                cursor_pos += 1;
                            }
                        }
                        KeyCode::Up => {
                            if hist_idx > 0 {
                                hist_idx -= 1;
                                input = entries[hist_idx].clone();
                                cursor_pos = input.len();
                            }
                        }
                        KeyCode::Down => {
                            if hist_idx + 1 < entries.len() {
                                hist_idx += 1;
                                input = entries[hist_idx].clone();
                                cursor_pos = input.len();
                            } else {
                                hist_idx = entries.len();
                                input.clear();
                                cursor_pos = 0;
                            }
                        }
                        KeyCode::Backspace => {
                            if cursor_pos > 0 {
                                input.remove(cursor_pos - 1);
                                cursor_pos -= 1;
                            }
                        }
                        KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::ALT) && c == 'c' => {
                            writeln!(stdout, "^C")?;
                            stdout.flush()?;
                            disable_raw_mode()?;
                            return Ok(String::new());
                        }
                        KeyCode::Char(c) => {
                            input.insert(cursor_pos, c);
                            cursor_pos += 1;
                        }
                        _ => {}
                    }
                    let (before, after) = input.split_at(cursor_pos);
                    write!(stdout, "\r\x1b[2K{}{}{}", prompt, before, after)?;
                    if after.len() > 0 {
                        write!(stdout, "\x1b[{}D", after.len())?;
                    }
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
        app.check_autosave_interval();

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
                    let ws_label = if active_ws && app.renaming_workspace {
                        " Rename ws... "
                    } else {
                        &format!(" {} ", ws.name)
                    };
                    tab_spans.push(ratatui::text::Span::styled(
                        ws_label.to_string(),
                        style,
                    ));
                }
                tab_spans.push(ratatui::text::Span::raw("   "));
                tab_spans.push(ratatui::text::Span::styled(
                    "Alt+M manage",
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
                let cell_label = if app.renaming_cell && active {
                    "Name:"
                } else if !pane.name.is_empty() {
                    &pane.name
                } else {
                    "Cell"
                };
                let title = format!("[{}/{}] {} ({}) ", i + 1, cell_count, cell_label, pane.active_language);
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
                    let dir_name = std::env::current_dir()
                        .ok()
                        .and_then(|d| d.file_name().map(|n| n.to_string_lossy().to_string()))
                        .unwrap_or_default();
                    let (before, after) = pane.input_buffer.split_at(pane.cursor_pos);
                    let prompt_line = if app.renaming_cell || app.renaming_workspace {
                        let label = if app.renaming_cell { " Name:" } else { " Workspace name:" };
                        ratatui::text::Line::from(vec![
                            ratatui::text::Span::styled("✎", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                            ratatui::text::Span::styled(label, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                            ratatui::text::Span::raw(" "),
                            ratatui::text::Span::raw(before),
                            ratatui::text::Span::raw(after),
                        ])
                    } else {
                        ratatui::text::Line::from(vec![
                            ratatui::text::Span::styled("➜ ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                            ratatui::text::Span::styled(format!("[{}]", pane.active_language), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                            ratatui::text::Span::styled(format!(" In {{{}}} ", pane.execution_count + 1), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                            ratatui::text::Span::styled(format!("({})", dir_name), Style::default().fg(Color::Yellow)),
                            ratatui::text::Span::raw(" "),
                            ratatui::text::Span::styled("❯", Style::default().fg(Color::Magenta)),
                            ratatui::text::Span::raw(" "),
                            ratatui::text::Span::raw(before),
                            ratatui::text::Span::raw(after),
                        ])
                    };
                    lines.push(prompt_line);

                    let cursor_y = lines.len() - 1;
                    let cursor_x_in_content = if app.renaming_cell || app.renaming_workspace {
                        let label = if app.renaming_cell { " Name:" } else { " Workspace name:" };
                        let mut count = "✎".chars().count();
                        count += label.chars().count();
                        count += 1;
                        count += before.chars().count();
                        count
                    } else {
                        let mut count = "➜ ".chars().count();
                        count += format!("[{}]", pane.active_language).chars().count();
                        count += format!(" In {{{}}} ", pane.execution_count + 1).chars().count();
                        count += format!("({})", dir_name).chars().count();
                        count += " ❯ ".chars().count();
                        count += before.chars().count();
                        count
                    };
                    let cursor_x = chunks[i].x + 1 + cursor_x_in_content as u16;
                    let cursor_y = chunks[i].y + 1 + cursor_y as u16;

                    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
                    f.render_widget(paragraph, chunks[i]);
                    f.set_cursor_position((cursor_x, cursor_y));
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
                            if app.renaming_cell || app.renaming_workspace {
                                app.cancel_rename();
                            } else {
                                app.auto_save();
                                app.running = false;
                                return Ok(());
                            }
                        }
                        KeyCode::Left if key.modifiers == KeyModifiers::ALT || key.modifiers == KeyModifiers::CONTROL | KeyModifiers::ALT => {
                            app.previous_workspace();
                        }
                        KeyCode::Right if key.modifiers == KeyModifiers::ALT || key.modifiers == KeyModifiers::CONTROL | KeyModifiers::ALT => {
                            app.next_workspace();
                        }
                        KeyCode::Left => {
                            let pane = app.current_pane_mut();
                            if pane.cursor_pos > 0 {
                                pane.cursor_pos -= 1;
                            }
                        }
                        KeyCode::Right => {
                            let pane = app.current_pane_mut();
                            if pane.cursor_pos < pane.input_buffer.len() {
                                pane.cursor_pos += 1;
                            }
                        }
                        KeyCode::Up if key.modifiers.contains(KeyModifiers::ALT) => {
                            app.renaming_cell = false;
                            app.renaming_workspace = false;
                            let ws = app.current_workspace_mut();
                            if ws.active_pane > 0 {
                                ws.active_pane -= 1;
                            }
                        }
                        KeyCode::Down if key.modifiers.contains(KeyModifiers::ALT) => {
                            app.renaming_cell = false;
                            app.renaming_workspace = false;
                            let ws = app.current_workspace_mut();
                            if ws.active_pane + 1 < ws.panes.len() {
                                ws.active_pane += 1;
                            }
                        }
                        KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                            app.renaming_cell = false;
                            app.renaming_workspace = false;
                            app.move_cell_up();
                        }
                        KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                            app.renaming_cell = false;
                            app.renaming_workspace = false;
                            app.move_cell_down();
                        }
                        KeyCode::Up => {
                            let pane = app.current_pane_mut();
                            if pane.history_index > 0 {
                                pane.history_index -= 1;
                                pane.input_buffer = pane.history[pane.history_index].clone();
                                pane.cursor_pos = pane.input_buffer.len();
                            }
                        }
                        KeyCode::Down => {
                            let pane = app.current_pane_mut();
                            if pane.history_index + 1 < pane.history.len() {
                                pane.history_index += 1;
                                pane.input_buffer = pane.history[pane.history_index].clone();
                            } else {
                                pane.history_index = pane.history.len();
                                pane.input_buffer.clear();
                                pane.cursor_pos = 0;
                            }
                        }
                        KeyCode::Tab => {
                            app.renaming_cell = false;
                            app.renaming_workspace = false;
                            let ws = app.current_workspace_mut();
                            if ws.active_pane + 1 < ws.panes.len() {
                                ws.active_pane += 1;
                            }
                        }
                        KeyCode::BackTab => {
                            app.renaming_cell = false;
                            app.renaming_workspace = false;
                            let ws = app.current_workspace_mut();
                            if ws.active_pane > 0 {
                                ws.active_pane -= 1;
                            }
                        }
                        KeyCode::Enter => {
                            if app.renaming_cell || app.renaming_workspace {
                                app.commit_rename();
                                continue;
                            }
                            let input = {
                                let p = app.current_pane_mut();
                                p.input_buffer.clone()
                            };
                            if let Some((target_lang, cmd)) = parse_lang_command(&input, &app.config) {
                                let state = app.state.clone();
                                let ws = &mut app.workspaces[app.active_workspace];
                                let idx = ws.ensure_pane(&target_lang, &app.config, state);
                                ws.active_pane = idx;
                                app.current_pane_mut().input_buffer = cmd.clone();
                                app.current_pane_mut().handle_input().await;
                                app.record_command(&target_lang, &cmd, &[]);
                                app.auto_save();
                            } else {
                                let input_parts: Vec<&str> = input.split_whitespace().collect();
                                if input_parts.len() == 2 && input_parts[0] == "noo" && app.config.contains_key(input_parts[1]) {
                                    let state = app.state.clone();
                                    let ws = &mut app.workspaces[app.active_workspace];
                                    let idx = ws.ensure_pane(input_parts[1], &app.config, state);
                                    ws.active_pane = idx;
                                    continue;
                                }
                                let lang = {
                                    let p = app.current_pane_mut();
                                    p.active_language.clone()
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
                                app.auto_save();
                            }
                        }
                        KeyCode::Backspace => {
                            let pane = app.current_pane_mut();
                            if pane.cursor_pos > 0 {
                                pane.input_buffer.remove(pane.cursor_pos - 1);
                                pane.cursor_pos -= 1;
                            }
                        }
                        KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::ALT) && key.modifiers.contains(KeyModifiers::SHIFT) => match c {
                            'W' => app.remove_workspace(),
                            'R' => app.toggle_rename_workspace(),
                            _ => {}
                        },
                        KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::ALT) => match c {
                            't' => app.add_cell(),
                            'w' => app.remove_cell(),
                            'n' => app.add_workspace(),
                            'r' => app.toggle_rename_cell(),
                            'm' => {
                                let _ = run_manage_tui(app).await;
                                enable_raw_mode()?;
                                execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
                            }
                            _ => {}
                        },
                        KeyCode::Char(c) => {
                            let pane = app.current_pane_mut();
                            pane.input_buffer.insert(pane.cursor_pos, c);
                            pane.cursor_pos += 1;
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
