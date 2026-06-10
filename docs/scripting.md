# Scripting

`.ns` files allow batch execution of code across multiple languages with cross-language variable sharing via the state bridge.

## Format

```
# Comments start with #
<language_key> <code>
```

Each line starts with a language key (matching an entry in `languages.json`) followed by a space and the code to execute. Bare lines inherit the language from the preceding line.

## Examples

### Basic

```ns
py print("hello from python")
js console.log("hello from js")
```

### Cross-language variable sharing

```ns
py x = 42
py name = "world"
js console.log("JS sees x =", x)
js console.log("JS sees name =", name)
js y = 99
js greeting = "hello from js"
py print("Python sees y =", y)
py print("Python sees greeting =", greeting)
```

Variables defined in one language are automatically injected into the next language's REPL via the state bridge.

### Inheriting language

```ns
py
x = 42
y = 10
print(x + y)
```

All three non-prefixed lines inherit `py` as their language.

## Execution

```
noo script.ns
```

Each line spawns a fresh REPL subprocess for its language, injects shared state, executes the code, and dumps state back. Output is printed to stdout.

## Limitations

- Each line spawns a separate subprocess — state persists via the bridge, not by keeping a session alive.
- Only non-underscore, non-builtin variables are shared across languages.
- The state bridge currently supports Python (`py`) and JavaScript (`js`).
