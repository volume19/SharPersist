//! SharPersist Rust Port
//!
//! Windows persistence toolkit for authorized security testing.
//! This library provides a safe Rust interface to Windows persistence mechanisms.

pub mod core;
pub mod ffi;
pub mod lib;
pub mod techniques;

pub use core::{config::PersistConfig, error::PersistError, Method};
pub use lib::args::CliArgs;
pub use lib::utils;

#[cfg(target_os = "windows")]
pub use ffi::windows_registry;
