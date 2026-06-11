//! # nooshell
//!
//! A multi-language REPL notebook and shell. Run interactive REPL sessions for
//! multiple languages in notebook-style workspaces with persistent command history.
//!
//! ## Main components
//!
//! - [`app`] — Top-level [`App`](app::App) and [`Workspace`](app::Workspace) structs
//! - [`pane`] — Notebook cell connected to a REPL subprocess
//! - [`execution`] — Subprocess lifecycle and async I/O
//! - [`bridge`] — Cross-language variable sharing via state injection/dump
//! - [`state`] — Thread-safe shared variable store
//! - [`store`] — Command history and session persistence
//! - [`config`] — Language configuration loading
//! - [`noorc`] — Startup config file parser
//! - [`script`] — `.ns` script parser and runner
//! - [`compile`] — Script-to-native-binary compiler

pub mod app;
pub mod bridge;
pub mod compile;
pub mod config;
pub mod execution;
pub mod noorc;
pub mod pane;
pub mod passthrough;
pub mod pty;
pub mod script;
pub mod shell_resolver;
pub mod state;
pub mod store;
pub mod terminal_bridge;
