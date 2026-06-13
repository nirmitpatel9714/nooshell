# Keybindings

## Notebook mode (`noo nbmode`)

### Navigation

| Key | Action |
| --- | --- |
| `Up` / `Down` | Navigate cells |
| `Left` / `Right` | Move cursor within input |
| `Tab` | Next cell |
| `Shift+Tab` | Previous cell |
| `Alt+Left` | Previous workspace |
| `Alt+Right` | Next workspace |

### Cell management

| Key | Action |
| --- | --- |
| `Alt+T` | New cell (inserts after active cell) |
| `Alt+W` | Remove cell |
| `Shift+Up` | Move cell up |
| `Shift+Down` | Move cell down |

### History

| Key | Action |
| --- | --- |
| `Alt+Up` | Previous command in cell history |
| `Alt+Down` | Next command in cell history |

### Execution

| Key | Action |
| --- | --- |
| `Enter` | Execute active cell |

### Renaming

| Key | Action |
| --- | --- |
| `Alt+R` | Rename active cell |
| `Alt+Shift+R` | Rename active workspace |

During rename mode, the input buffer shows the current name. Press `Enter` to commit or `Esc` to cancel.

### Workspace management

| Key | Action |
| --- | --- |
| `Alt+N` | New workspace |
| `Alt+Shift+W` | Remove active workspace |

### Management

| Key | Action |
| --- | --- |
| `Alt+M` | Open Management TUI |

### Exit

| Key | Action |
| --- | --- |
| `Esc` | Exit notebook mode (cancels rename first if active) |

## CLI mode (`noo`)

### Navigation

| Key | Action |
| --- | --- |
| `Up` / `Down` | Navigate global command history |
| `Left` / `Right` | Move cursor within input |
| `Backspace` | Delete character before cursor |

### Execution

| Key | Action |
| --- | --- |
| `Enter` | Execute command |
| `Alt+C` | Cancel (sends empty input) |

### Language switching

Type a language key (matching `languages.json`) and press Enter to switch to that language's REPL.

### Cross-language execution

```
py print("hello")    # Run Python code from any language
js console.log("hi")  # Run JavaScript code from any language
```

### Subcommands

From within CLI mode, prefix with `noo`:

| Command | Action |
| --- | --- |
| `noo manage` | Open Management TUI |
| `noo history` | Show command history |
| `noo sessions` | List saved sessions |
| `noo clearc` | Clear command history |
| `noo delses <id>` | Delete a saved session |
| `noo <lang>` | Add a new pane for that language |

## Management TUI (`noo manage`)

| Key | Action |
| --- | --- |
| `Tab` / `Shift+Tab` | Switch between Sessions / History tabs |
| `Up` / `Down` | Navigate list |
| `d` | Delete selected session (Sessions tab) or clear history (History tab) |
| `Esc` | Return to previous mode |
