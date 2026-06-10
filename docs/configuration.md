# Configuration

## `languages.json`

Defines the REPL commands for each language. Located in the project root (or working directory).

```json
{
  "py": {
    "cmd": "python",
    "args": ["-i"],
    "mode": "repl"
  },
  "js": {
    "cmd": "node",
    "args": ["-i"],
    "mode": "repl"
  },
  "ps": {
    "cmd": "powershell",
    "args": ["-NoExit", "-Command", "-"],
    "mode": "repl"
  }
}
```

### Fields

| Field    | Type            | Description                                  |
| ---      | ---             | ---                                          |
| `cmd`    | string          | The executable to run                        |
| `args`   | array of string | Arguments passed to the executable           |
| `mode`   | string           | Currently always `"repl"` (future: compile)  |

### Adding a language

```json
"rb": {
  "cmd": "irb",
  "args": [],
  "mode": "repl"
}
```

The key (`"rb"`) becomes the language alias used in `lang(code)` syntax and language switching.

## `noorc`

A startup config file at `%APPDATA%/nooshell/noorc` (Windows) or `$HOME/.config/nooshell/noorc` (Unix).

### Syntax

```
# Comments start with #
language "py"
alias hi = "print('hello')"
print("this runs on startup")
```

### Directives

| Directive   | Description                                         |
| ---         | ---                                                 |
| `language`  | Set the default language for the initial pane       |
| `alias`     | Define a shortcut that expands to a full command     |
| *(any line)* | Lines without a directive are run as startup commands |

### Example

```
language js
alias greet = "console.log('hello world')"
greet
```

This starts the session in JavaScript mode, defines `greet` as an alias for `console.log('hello world')`, and runs it on startup.

## Environment variables

- `NOO_STATE_FILE` — set automatically on REPL subprocesses, points to a temp file used for state bridge serialization
- `APPDATA` / `HOME` — determines the data directory for history, sessions, and noorc
