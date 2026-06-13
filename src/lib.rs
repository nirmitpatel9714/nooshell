//! # NooBook
//!
//! A multi-language REPL terminal notebook. Run interactive REPL sessions for
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
//! - [`highlight`] — Syntax highlighting
//! - [`lsp`] — LSP client for semantic tokens

pub mod app;
pub mod bridge;
pub mod config;
pub mod execution;
pub mod highlight;
pub mod lsp;
pub mod noorc;
pub mod pane;
pub mod state;
pub mod store;
