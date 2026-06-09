# nooshell

A multi-language REPL notebook and shell. Run interactive REPL sessions for multiple languages in notebook-style workspaces with persistent command history.

## Features

- **Notebook mode** (`noo nbmode`) — TUI with multiple workspaces, each containing notebook cells with independent REPL sessions
- **CLI mode** (`noo`) — single-pane REPL with bash-like arrow-key history navigation
- **Multi-language** — Python, JavaScript (Node.js), configurable via `languages.json`
- **Workspaces** — horizontal tabs, each with its own set of vertically stacked notebook cells
- **Cell management** — add (`Alt+T`), remove (`Alt+W`), reorder (`Shift+Up`/`Shift+Down`)
- **Command history** — persists across sessions to `%APPDATA%\nooshell\history.json`
- **Session management** — save/restore workspace state; manage from TUI (`Alt+M`) or CLI

## Usage

```
noo                 CLI mode
noo nbmode          Notebook TUI
noo manage          Management TUI
noo history         Show command history
noo clearc          Clear command history
noo sessions        List saved sessions
noo delses <id>     Delete a session
```

### Notebook keybindings

| Key | Action |
|-----|--------|
| `Left` / `Right` | Switch workspace |
| `Up` / `Down` | Navigate cells |
| `Shift+Up` / `Shift+Down` | Move cell |
| `Alt+T` | New cell |
| `Alt+W` | Remove cell |
| `Alt+N` | New workspace |
| `Alt+Up` / `Alt+Down` | History in cell |
| `Enter` | Execute cell |
| `Alt+M` | Management TUI |
| `Esc` | Exit |

## Configuration

Edit `languages.json` to add or change language REPLs:

```json
{
  "py": { "cmd": "python", "args": ["-i"], "mode": "repl" },
  "js": { "cmd": "node",  "args": ["-i"], "mode": "repl" }
}
```

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) (1.85+)

### PowerShell (Windows)

```powershell
.\scripts\windows\install.ps1
```

This builds the binary, copies it to `~\.noo\bin`, and adds that directory to your user `PATH`. Restart your terminal and `noo` is available globally.

### Bash (Git Bash / WSL / Linux / macOS)

```sh
chmod +x scripts/unix/install.sh
./scripts/unix/install.sh
```

This builds the binary, copies it to `~/.noo/bin`, and adds it to your shell's PATH in `.bashrc` / `.zshrc`.

### Uninstall

| Platform | Command |
|----------|---------|
| Windows  | `.\scripts\windows\uninstall.ps1` |
| Unix     | `./scripts/unix/uninstall.sh` |

### Build manually

```sh
cargo build --release
# Binary is at target/release/noo.exe (Windows) or target/release/noo (Unix)
```
