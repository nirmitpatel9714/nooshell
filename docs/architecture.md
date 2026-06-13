# Architecture

NooBook is a multi-language REPL terminal notebook built in Rust with `ratatui` for the TUI and `crossterm` for terminal handling. It runs subprocesses for each language REPL and communicates with them via stdin/stdout.

## High-level overview

```
┌─────────────────────────────────────────────────────┐
│  main.rs                                            │
│  ───────                                            │
│  Parses CLI args, loads config & noorc, dispatches  │
│  to CLI mode, Notebook TUI, or Management TUI.       │
└──────────┬──────────────────────────────────────────┘
           │
     ┌─────┴─────────────────────┐
     │  App (app.rs)             │
     │  ────────────             │
     │  Vec<Workspace>           │
     │  ConfigMap                │
     │  SharedState              │
     └─────┬─────────────────────┘
           │
     ┌─────┴─────────────────────┐
     │  Workspace (app.rs)       │
     │  ───────────────          │
     │  Vec<Pane>                │
     │  active_pane              │
     │  name                     │
     └─────┬─────────────────────┘
           │
     ┌─────┴─────────────────────┐
     │  Pane (pane.rs)           │
     │  ────────────             │
     │  ProcessSession           │
     │  input_buffer             │
     │  output_lines             │
     │  history                  │
     │  SharedState              │
     └─────┬─────────────────────┘
           │
      ┌─────┴─────────────────────┐
      │  ProcessSession           │
      │  (execution.rs)           │
      │  ──────────────           │
      │  Child process (REPL)     │
      │  mpsc channels for I/O    │
      └───────────────────────────┘
```

## Core components

### `App` — `src/app.rs`
The top-level application struct. Holds:
- A list of `Workspace`s (each is a notebook tab)
- The active workspace index
- A `ConfigMap` of language configurations
- A `SharedState` that is shared across all panes for cross-language variable bridging

### `Workspace` — `src/app.rs`
Represents a notebook tab containing:
- A vector of `Pane`s (cells)
- An index to the active pane
- A user-visible name

### `Pane` — `src/pane.rs`
A single notebook cell connected to a REPL process. Each pane:
- Has a `ProcessSession` wrapping a child process
- Maintains an `input_buffer` for user input and `output_lines` for REPL output
- Tracks command `history` per cell
- Has a `SharedState` reference for the state bridge
- Supports built-in commands (`clear`, `cd`, `ls`, `exit`, `noo`)

### `ProcessSession` — `src/execution.rs`
Manages a child REPL process:
- Spawns a subprocess with piped stdin/stdout/stderr
- Uses `mpsc` channels for async communication
- Three Tokio tasks: one writing to stdin, two reading from stdout and stderr

## Modes

### CLI mode (`noo`)
Single-pane REPL with arrow-key history navigation. Parses `lang(code)` or `lang code` syntax to dispatch to specific languages. Supports all `noo` subcommands from within the REPL.

### Notebook TUI (`noo nbmode`)
Full-screen terminal UI with:
- Tab bar for workspace switching
- Vertically stacked notebook cells
- Per-cell REPL sessions with independent history
- Cell add/remove/reorder/rename
- Renamable workspaces
- Autosave every 10 seconds

### Management TUI (`noo manage`)
TUI for viewing and managing saved sessions and command history.

## Data persistence

### Command history
Stored at `%APPDATA%/NooBook/history.json` (Windows) or `$HOME/NooBook/history.json` (Unix). Each command is recorded with a language tag, timestamp, and output preview.

### Session persistence
Stored at `%APPDATA%/NooBook/sessions.json`. Full workspace state (cells, history, outputs, cursor positions) is serialized. Sessions can be saved/restored from the Management TUI. An `_autosave` session is saved every 10 seconds during notebook mode and offered for restoration on startup.

### Noorc — `src/noorc.rs`
A config file at `%APPDATA%/NooBook/noorc` (Windows) or `$HOME/NooBook/noorc` (Unix) that sets:
- Default language
- Command aliases
- Startup commands (run on boot)

## Threading model

- `async fn main()` with `#[tokio::main]`
- Each REPL process spawns 3 Tokio tasks for I/O
- `poll_output()` on each pane drains the output `mpsc` channel (non-blocking)
- The TUI event loop polls for keyboard events every 50ms
- State bridge uses `Arc<Mutex<Map>>` for shared mutable state across panes
