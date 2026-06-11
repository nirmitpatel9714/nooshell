use crate::pty;
use crate::shell_resolver;
use crate::terminal_bridge;
use std::io::Write;
use std::time::Duration;

pub struct PassThroughConfig {
    pub shell_name: String,
    pub cwd: Option<String>,
    pub login: bool,
    pub config_file: Option<String>,
}

impl Default for PassThroughConfig {
    fn default() -> Self {
        PassThroughConfig {
            shell_name: "bash".to_string(),
            cwd: None,
            login: false,
            config_file: None,
        }
    }
}

fn get_terminal_size() -> Result<(u16, u16, u16, u16), Box<dyn std::error::Error>> {
    match crossterm::terminal::window_size() {
        Ok(ws) => Ok((ws.columns, ws.rows, ws.width, ws.height)),
        Err(_) => {
            let (cols, rows) = crossterm::terminal::size()?;
            Ok((cols, rows, 0, 0))
        }
    }
}

pub fn run_passthrough(config: &PassThroughConfig) -> Result<(), Box<dyn std::error::Error>> {
    let shell = shell_resolver::resolve_shell(&config.shell_name).ok_or_else(|| {
        format!(
            "Shell '{}' was not found on this system.",
            config.shell_name
        )
    })?;

    eprint!("\r\n[noo passthrough] starting {} at {}\r\n", shell.display_name, shell.path.display());

    let (cols, rows, _pixel_w, _pixel_h) = get_terminal_size()?;

    let session = pty::create_pty_session(
        shell.path.to_str().ok_or("Invalid shell path")?,
        &shell.args,
        cols,
        rows,
    )?;

    let reader = session.try_clone_reader()?;
    let writer = session.take_writer()?;

    eprint!("\r\n[noo passthrough] attached, forwarding I/O (type 'exit' to return)\r\n");
    std::io::stderr().flush().ok();

    terminal_bridge::enter_passthrough_mode()?;

    let bridge = terminal_bridge::TerminalBridge::start(reader, writer);

    let mut last_cols = cols;
    let mut last_rows = rows;

    loop {
        if !bridge.is_running() {
            break;
        }

        if let Some(code) = session.try_wait_child() {
            bridge.signal_stop();
            eprint!("\r\n[noo passthrough] shell exited (code {})\r\n", code);
            break;
        }

        if let Ok((new_cols, new_rows, _new_pw, _new_ph)) = get_terminal_size() {
            if new_cols != last_cols || new_rows != last_rows {
                last_cols = new_cols;
                last_rows = new_rows;
                let _ = session.resize(new_cols, new_rows);
            }
        }

        std::thread::sleep(Duration::from_millis(50));
    }

    drop(bridge);
    drop(session);

    terminal_bridge::exit_passthrough_mode()?;

    let _ = crossterm::terminal::size();
    println!();

    Ok(())
}
