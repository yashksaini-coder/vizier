//! Vizier - Rust Code Inspector Library
//!
//! A comprehensive library for analyzing Rust code, parsing cargo metadata,
//! and providing a beautiful TUI for code inspection.

pub mod analyzer;
pub mod app;
pub mod config;
pub mod crates_io;
pub mod error;
pub mod ui;
pub mod utils;

pub use app::App;
pub use error::{Result, VizierError};
