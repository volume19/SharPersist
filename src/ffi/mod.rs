//! Foreign function interface wrappers for Windows APIs

#[cfg(target_os = "windows")]
pub mod windows_registry;

#[cfg(target_os = "windows")]
pub mod windows_service;
