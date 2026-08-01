//! oxiline-core — pure Rust library for OxiLine.
//!
//! Holds the SQLite schema, domain types, occurrence/materialize logic, and all
//! CRUD. Knows nothing about Tauri or clap. Both `oxiline-app` (GUI) and
//! `oxiline-cli` call into this crate.

pub mod activities;
pub mod cards;
pub mod categories;
pub mod db;
pub mod error;
pub mod model;
pub mod paths;
pub mod plan;
pub mod reports;
pub mod routine_groups;
pub mod routines;
pub mod settings;
pub mod tasks;
pub mod timeline;
pub mod util;

pub use db::open_and_migrate;
pub use error::{CoreError, ErrorCode, Result};
