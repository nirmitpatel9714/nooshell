# NooBook

A multi-language REPL terminal notebook built in Rust. Run interactive REPL sessions
for multiple languages in notebook-style workspaces with persistent command history
and cross-language variable sharing.

## Features

- **Notebook mode** (`noo nbmode`) — full TUI with multiple workspace tabs, each
  containing vertically stacked notebook cells with independent REPL sessions
- **CLI mode** (`noo`) — single-pane REPL with bash-like arrow-key history navigation
- **Management TUI** (`noo manage`) — view and manage saved sessions and command history
- **Script mode** (`noo script.ns`) — batch execution of `.ns` script files
- **Compile mode** (`noo compile script.ns`) — compile `.ns` scripts into standalone
  native binaries (supports cross-compilation for Windows, Linux, macOS)
- **Multi-language** — Python, JavaScript (Node.js), PowerShell, bash, and any language
  configurable via `languages.json`
- **LSP-based syntax highlighting** — configure an LSP server per language for semantic
  token highlighting (e.g., `clangd` for C++)
- **Passthrough terminal** (`noo pass`) — full PTY-backed shell session for
  interactive programs (vim, htop, etc.)
- **Cross-language variable sharing** — variables defined in one language are
  automatically available in other REPLs via the state bridge
- **Workspaces** — horizontal tabs, each with its own set of vertically stacked cells
- **Cell management** — add, remove, reorder, and rename notebook cells
- **Command history** — persists across sessions with timestamps and output previews
- **Session persistence** — save/restore full workspace state (cells, history, outputs)
  with automatic 10-second autosave

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) (edition 2024, tested with 1.85+)

### Windows (PowerShell)

```powershell
.\scripts\install scripts\windows\install.ps1
```

Builds the binary, copies it to `~\.noo\bin`, and adds that directory to your user `PATH`.
Restart your terminal and `noo` is available globally.

### Unix (Git Bash / WSL / Linux / macOS)

```sh
chmod +x "scripts/install scripts/unix/install.sh"
./scripts/install scripts/unix/install.sh
```

Builds the binary, copies it to `~/.noo/bin`, and adds it to your shell's `PATH`
in `.bashrc` / `.zshrc`.

### Build manually

```sh
cargo build --release
# Binary at target/release/noo.exe (Windows) or target/release/noo (Unix)
```

### Uninstall

| Platform | Command |
|----------|---------|
| Windows  | `.\scripts\install scripts\windows\uninstall.ps1` |
| Unix     | `./scripts/install scripts/unix/uninstall.sh` |

## Usage

```
noo                          CLI mode (single-pane REPL)
noo nbmode                   Notebook TUI (multi-workspace)
noo manage                   Management TUI
noo history                  Show command history
noo sessions                 List saved sessions
noo clearc                   Clear command history
noo delses <id>              Delete a saved session
noo script.ns                Run a .ns script
noo pass                     Passthrough terminal mode (default shell)
noo pass bash                Passthrough to bash
noo pass ps                  Passthrough to PowerShell
noo compile script.ns        Compile script to native binary
noo compile script.ns --linux  Cross-compile for Linux
noo compile script.ns --mac    Cross-compile for macOS
noo compile script.ns --windows Cross-compile for Windows
```

### Inline language switching (CLI + Notebook)

```
py (print("hello from python"))
js (console.log("hello from javascript"))
ps (Write-Host "hello from powershell")
```

This dispatches to any language, creating a new pane if one doesn't exist.

### Cross-language execution

```
py  x = 42
js  console.log(x)  
# prints 42
```

Variables set in one language are automatically injected into others via the
state bridge.

## Keybindings

### Notebook mode (`noo nbmode`)

| Key | Action |
| --- | --- |
| `Up` / `Down` | Navigate cells |
| `Left` / `Right` | Move cursor within input |
| `Tab` / `Shift+Tab` | Next / previous cell |
| `Alt+Left` / `Alt+Right` | Previous / next workspace |
| `Alt+T` | New cell |
| `Alt+W` | Remove cell |
| `Alt+N` | New workspace |
| `Alt+Shift+W` | Remove workspace |
| `Shift+Up` / `Shift+Down` | Move cell up / down |
| `Alt+Up` / `Alt+Down` | Previous / next command in cell history |
| `Enter` | Execute active cell |
| `Alt+R` | Rename active cell |
| `Alt+Shift+R` | Rename active workspace |
| `Alt+M` | Open Management TUI |
| `Esc` | Exit (or cancel rename) |

### CLI mode (`noo`)

| Key | Action |
| --- | --- |
| `Up` / `Down` | Navigate global command history |
| `Left` / `Right` | Move cursor within input |
| `Enter` | Execute command |
| `Alt+C` | Cancel (sends empty input) |

### Management TUI (`noo manage`)

| Key | Action |
| --- | --- |
| `Tab` / `Shift+Tab` | Switch between Sessions / History |
| `Up` / `Down` | Navigate list |
| `d` | Delete selected session or clear history |
| `Esc` | Return to previous mode |

## Project structure

```
src/
├── main.rs        Entry point, CLI parsing, TUI rendering
├── lib.rs         Module declarations, crate-level docs
├── app.rs         App and Workspace structs
├── pane.rs        Pane (notebook cell) struct
├── execution.rs   ProcessSession — REPL subprocess lifecycle
├── bridge.rs      State bridge — cross-language variable sharing
├── config.rs      Language config loading (languages.json)
├── state.rs       SharedState — thread-safe variable store
├── store.rs       History & session persistence on disk
├── noorc.rs       Noorc config file parser
├── script.rs      .ns script parser and runner
└── compile.rs     Script-to-native-binary compiler

docs/
├── architecture.md    High-level architecture and threading model
├── configuration.md   languages.json and noorc format reference
├── keybindings.md     Full keybinding reference
├── passthrough.md     PTY passthrough mode, shell resolution, terminal bridge
├── persistence.md     Command history & session persistence data model
├── scripting.md       .ns scripting guide
└── state-bridge.md    Deep dive on cross-language variable sharing
```

## Passthrough

See [docs/passthrough.md](docs/passthrough.md) for the PTY-backed passthrough
terminal mode, shell resolution, and configuration.

## Persistence

See [docs/persistence.md](docs/persistence.md) for command history and session
persistence data model, autosave, and management commands.

## Configuration

See [docs/configuration.md](docs/configuration.md) for details on `languages.json`
and `noorc` configuration.

## Scripting

See [docs/scripting.md](docs/scripting.md) for the `.ns` scripting format.

## Development

### Running tests

```sh
cargo test          # unit tests (state bridge, parsing)
cargo test -- --nocapture  # with debug output
```

The state bridge tests in `src/bridge.rs` require Python and Node.js installed.

### Running the test script

```sh
cargo run -- test.ns
```

This exercises all major features: basic execution, conditionals, concurrency,
large state, edge cases, and load tests.

### Building the TUI

```sh
cargo run -- nbmode         # debug build
cargo run --release -- nbmode  # release build
```

### Verifying

```sh
cargo clippy   # lint checks
```

## Documentation

To generate and open the Rust API docs:

```sh
cargo doc --no-deps --open
```

## Architecture

```
┌─────────────────────────────────────────────┐
│  main.rs — CLI dispatch                     │
│  CLI mode | Notebook TUI | Management TUI   │
│  Script mode | Compile mode                 │
└──────────┬──────────────────────────────────┘
           │
     ┌─────┴──────────────┐
     │  App (app.rs)      │
     │  Vec<Workspace>    │
     │  ConfigMap         │
     │  SharedState       │
     └─────┬──────────────┘
           │
     ┌─────┴──────────────┐
     │  Workspace (app.rs)│
     │  Vec<Pane>         │
     └─────┬──────────────┘
           │
     ┌─────┴──────────────┐
     │  Pane (pane.rs)    │
     │  ProcessSession    │
     │  SharedState ref   │
     └─────┬──────────────┘
           │
     ┌─────┴──────────────┐
     │  ProcessSession    │
     │  (execution.rs)    │
     │  Child process     │
     │  mpsc I/O channels │
     └────────────────────┘
```

Each REPL subprocess spawns 3 Tokio tasks (stdin writer, stdout reader,
stderr reader). The state bridge injects variables before user code and
dumps changes back afterward, using the `__NS_SYNC__` prefix protocol.

## License

This project is open source. See the [LICENSE](LICENSE) file.
