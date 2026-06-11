# Persistence

nooshell persists two kinds of data to disk: **command history** and **workspace sessions**. Both are stored as JSON files in a platform-specific data directory.

## Data directory

| Platform | Path |
| --- | --- |
| Windows | `%APPDATA%\nooshell\` |
| Unix    | `~/.local/share/nooshell/` (falls back to `$HOME/nooshell/`) |

Set `APPDATA` (Windows) or `HOME` (Unix) to override.

## Command history — `history.json`

Every command executed in CLI mode or the notebook TUI is recorded with metadata.

### Record structure

```json
{
  "id": 1747427123456789000,
  "session_key": "",
  "language": "py",
  "command": "print('hello')",
  "timestamp": "1747427123456",
  "output_preview": "hello"
}
```

| Field            | Type    | Description                              |
| ---------------- | ------- | ---------------------------------------- |
| `id`             | u64     | Nanosecond-precision unique ID           |
| `session_key`    | string  | Reserved for session grouping            |
| `language`       | string  | Language key (`py`, `js`, `ps`, etc.)    |
| `command`        | string  | The raw command text                     |
| `timestamp`      | string  | Milliseconds since Unix epoch            |
| `output_preview` | string  | First line of output (for quick lookup)  |

### CLI commands

```
noo history           # Show last 50 commands (reverse chronological)
noo clearc            # Delete all command history
```

### Storage

`src/store.rs` — `HistoryStore` wraps a `Vec<CommandRecord>`. On every `push_command()`, the full store is re-serialized to `history.json`. For typical usage (thousands of entries) this is fast enough.

## Session persistence — `sessions.json`

Full workspace state can be saved and restored. This includes all cells, their input buffers, output history, cursor positions, and execution counts.

### Record structure

```json
{
  "id": "_autosave",
  "name": "My Session",
  "created_at": "1747427123456",
  "updated_at": "1747427123999",
  "workspaces": [
    {
      "name": "Default",
      "active_pane": 0,
      "cells": [
        {
          "name": "Data Prep",
          "active_language": "py",
          "history": ["x = 42", "print(x)"],
          "execution_count": 5,
          "output_lines": ["42"],
          "input_buffer": "print(x + 1)",
          "cursor_pos": 0
        }
      ]
    }
  ]
}
```

### Session management

| Command | Action |
| --- | --- |
| `noo sessions` | List all saved sessions |
| `noo delses <id>` | Delete a session |
| `noo manage` | Management TUI — view, delete sessions |

### Autosave

In notebook mode (`noo nbmode`), the current workspace state is automatically saved to a session with the `_autosave` ID every **10 seconds** (see `App::auto_save()` in `src/app.rs`).

On startup, if an autosave exists, nooshell prompts:

```
Autosaved session found. Restore? [Y/n]:
```

Answering `Y` (or pressing Enter) restores the full workspace state. Answering `n` starts fresh and the autosave is retained for later restoration.

### Storage

`src/store.rs` — `SessionStore` wraps a `Vec<SessionRecord>`. Read and written in full via `serde_json`. Sessions are identified by a string `id` (the `_autosave` sentinel or a user-visible name). The `update_session()` function inserts or replaces by ID.

## Data flow

```
┌─────────────┐     load_history()     ┌──────────────┐
│  history.json│ ◄───────────────────► │  HistoryStore │
└─────────────┘   push_command()        │  (in memory)  │
                   save_history()       └──────────────┘

┌──────────────┐    load_sessions()    ┌──────────────┐
│ sessions.json│ ◄───────────────────► │ SessionStore  │
└──────────────┘  update_session()     │  (in memory)  │
                   delete_session()    └──────────────┘
                          │
                    auto_save()  ◄── timer (every 10s)
```

## Limitations

- Full-file write on every update — large histories may cause disk churn (mitigated by small payload size).
- No automatic history pruning — use `noo clearc` manually.
- Session format is tied to the current data model; restoring sessions from older versions may fail if the schema changes.
