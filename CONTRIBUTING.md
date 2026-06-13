# Contributing

Thanks for considering contributing to NooBook!

## Code of conduct

Be respectful, constructive, and inclusive. This is a small open-source project —
treat others the way you'd like to be treated.

## How to contribute

### Reporting bugs

Open an issue with:
- A clear, descriptive title
- Steps to reproduce
- Expected vs actual behavior
- Your platform (Windows/Linux/macOS) and Rust version (`rustc --version`)

### Suggesting features

Open an issue describing the feature, why it's useful, and how it might work.
Small, focused proposals are easier to discuss and implement.

### Submitting pull requests

1. Fork the repository.
2. Create a feature branch (`git checkout -b feature/my-change`).
3. Make your changes.
4. Run the tests and ensure they pass:
   ```sh
   cargo test
   ```
5. Run clippy and fix any warnings:
   ```sh
   cargo clippy
   ```
6. Commit with a clear, concise message describing *what* and *why*.
7. Push and open a pull request.

## Development setup

```sh
git clone https://github.com/anomalyco/NooBook
cd NooBook
cargo build
```

The minimal dependencies are Rust 1.85+ and the packages listed in `Cargo.toml`.
No system libraries are required.

### Running the notebook TUI

```sh
cargo run -- nbmode
```

Some tests in `src/bridge.rs` require Python 3 and Node.js on your `PATH`.

## Code style

- Follow the existing code patterns (look at neighboring code before writing).
- Avoid adding comments unless the code's intent is not obvious from reading it.
- Write `///` doc comments for public items.
- Use `cargo fmt` before committing (default Rust formatting).
- Keep functions focused and reasonably short.

## Documentation

- Update existing docs in `docs/` if you change behavior.
- If you add a new feature, add or update the relevant `.md` file.
- Run `cargo doc --no-deps --open` to preview the Rust API docs.

## Project structure

| Directory | Contents |
|-----------|----------|
| `src/` | Rust source code (library + binary) |
| `docs/` | Markdown documentation |
| `scripts/` | Install/uninstall scripts (Windows + Unix) |

See [docs/architecture.md](docs/architecture.md) for a full component overview.

## Questions?

Open a discussion or issues for any questions.
