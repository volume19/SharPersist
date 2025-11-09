//! Persistence technique implementations

#[cfg(target_os = "windows")]
pub mod registry;

#[cfg(target_os = "windows")]
pub mod service;

pub mod keepass;
pub mod schtask;
pub mod schtask_backdoor;
pub mod startup_folder;
pub mod tortoisesvn;
