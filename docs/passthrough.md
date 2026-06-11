# Passthrough Terminal Mode

`noo pass` (and `noo pass <shell>`) launches a full-screen terminal emulator for a native shell (bash, PowerShell, etc.) using a pseudo-terminal (PTY). Unlike the REPL notebook cells, passthrough mode gives you the complete, unmodified shell experience — including interactive programs like `vim`, `htop`, `git log`, etc.

## Usage

```
noo pass                # Auto-detect and launch the default shell
noo pass bash           # Launch bash
noo pass ps             # Launch PowerShell
noo pass pwsh           # Launch PowerShell Core
```

From the CLI REPL mode:

```
noo pass                # Same — passthrough to default shell
```

When the shell exits (via `exit` or `Ctrl+D`), you return to the nooshell REPL.

### Supported shells

| Alias   | Description           | Executables searched                |
| ------- | --------------------- | ----------------------------------- |
| `bash`  | Bourne Again SHell    | `bash`, `sh`                        |
| `ps`    | PowerShell            | `pwsh`, `powershell`, `powershell.exe` |

Configured in [`config/sh-languages.json`](../config/sh-languages.json).

## How it works

### 1. Shell resolution — `src/shell_resolver.rs`

The `resolve_shell()` function looks up the shell alias in `sh-languages.json`, then searches `$PATH` for each listed executable. On Windows, it also checks common installation paths under `Program Files` and `LOCALAPPDATA` (for Windows App aliases).

```rust
pub fn resolve_shell(name: &str) -> Option<ResolvedShell>
```

Returns a `ResolvedShell` containing the resolved executable `path`, display name, and CLI arguments. If no shell is specified, `detect_default_shell()` checks `$SHELL` on Unix or tries `ps` then `bash` on Windows.

### 2. PTY session — `src/pty.rs`

Wraps the [`portable-pty`](https://crates.io/crates/portable-pty) crate to create a cross-platform pseudo-terminal:

```rust
pub fn create_pty_session(
    shell_path: &str,
    shell_args: &[String],
    cols: u16,
    rows: u16,
    cwd: Option<&str>,
) -> Result<PtySession, Box<dyn std::error::Error>>
```

`PtySession` provides:
- `try_clone_reader()` — read PTY output (what the shell prints)
- `take_writer()` — write user input to the PTY
- `resize(cols, rows)` — forward terminal resize events
- `try_wait_child()` — non-blocking child exit check
- `is_alive()` — check if the child process is still running
- `shutdown()` — wait for the child to exit and clean up

The `PtySession` implements `Drop` so the child is always reaped.

### 3. Terminal bridge — `src/terminal_bridge.rs`

`TerminalBridge` connects the PTY to the real terminal with two background threads:

**PTY → stdout thread:** Reads from the PTY reader and writes to `stdout`. This renders the shell's output (prompts, command output, TUI programs) directly on screen.

**stdin → PTY thread:** Reads from the real keyboard and writes to the PTY writer. On Windows it uses `CreateFileA` / `ReadFile` on `CONIN$` to capture raw input. On Unix it uses `poll()` on stdin.

Both threads check an `AtomicBool` flag and exit cleanly when signaled.

### 4. Passthrough orchestrator — `src/passthrough.rs`

`run_passthrough()` ties everything together:

1. Resolves the shell executable
2. Gets the current terminal size
3. Creates a `PtySession`
4. Enables raw mode via `crossterm`
5. Starts the `TerminalBridge` to forward I/O
6. Loops every 50ms checking for terminal resize events and child exit
7. On exit, disables raw mode, drops the bridge, cleans up the session

### 5. Raw mode handling

Before passthrough starts, `enter_passthrough_mode()` enables crossterm raw mode so keystrokes pass through to the shell unmodified. On exit, `exit_passthrough_mode()` restores the terminal.

## Architecture diagram

```
┌─────────────────────────────────────────────────────────┐
│  noo pass                                               │
│  ────────                                               │
│  shell_resolver::resolve_shell("bash")                  │
│  pty::create_pty_session(...)                           │
│  terminal_bridge::enter_passthrough_mode()              │
│  TerminalBridge::start(reader, writer)                  │
│                                                         │
│  ┌─────────────────────────────────────┐                │
│  │  TerminalBridge                     │                │
│  │  ───────────────                    │                │
│  │  Thread 1: PTY → stdout            │                │
│  │  Thread 2: stdin → PTY             │                │
│  └─────────────────────────────────────┘                │
│                                                         │
│  loop: resize check · child exit check                  │
│  terminal_bridge::exit_passthrough_mode()               │
└─────────────────────────────────────────────────────────┘
```

## Shell configuration — `config/sh-languages.json`

```json
{
  "bash": {
    "executables": ["bash", "sh"],
    "args": [],
    "description": "Bourne Again SHell"
  },
  "ps": {
    "executables": ["pwsh", "powershell", "powershell.exe"],
    "args": ["-NoLogo"],
    "description": "PowerShell"
  }
}
```

### Fields

| Field          | Type            | Description                                    |
| -------------- | --------------- | ---------------------------------------------- |
| `executables`  | array of string | Executable names to search on `$PATH`          |
| `args`         | array of string | CLI arguments passed to the shell              |
| `description`  | string          | Human-readable name for display                |

## Cross-platform notes

- **Windows:** Uses `CreateFileA` on `CONIN$` for raw input reading (required because `std::io::stdin()` in PowerShell 5.1 can't read raw keystrokes through the PTY). Searches common `Program Files` paths for Git Bash and PowerShell.
- **Unix:** Uses `poll()` on `stdin`'s raw file descriptor via `libc`. `$SHELL` is used for default shell detection.
- **Resizing:** Terminal resize events are polled every 50ms and forwarded to the PTY via `resize()`.

## Limitations

- Only one passthrough session at a time (it takes over the full terminal).
- Shell configuration is static — add new shells by editing `sh-languages.json` and recompiling.
- Windows `CONIN$` raw read may behave differently across terminal emulators (Windows Terminal, ConHost, etc.).
