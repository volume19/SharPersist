//! Utility functions for persistence operations

use crate::core::error::{PersistError, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Compute SHA256 hash of a file
///
/// Equivalent to C# Utils.SHA256CheckSum
pub fn sha256_checksum<P: AsRef<Path>>(path: P) -> Result<String> {
    let mut file = File::open(path.as_ref())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    let hash = hasher.finalize();
    Ok(hex::encode(hash).to_uppercase())
}

/// Check if the current user has administrative privileges
///
/// Equivalent to C# Utils.IsUserAnAdmin()
///
/// # Platform Support
/// - Windows: Checks token elevation level
/// - Linux/macOS: Returns false (not supported)
#[cfg(target_os = "windows")]
pub fn is_user_admin() -> bool {
    use windows::Win32::Foundation::BOOL;
    use windows::Win32::Security::IsUserAnAdmin;

    // SAFETY: IsUserAnAdmin is a safe Windows API call with no preconditions
    unsafe { IsUserAnAdmin() == BOOL(1) }
}

#[cfg(not(target_os = "windows"))]
pub fn is_user_admin() -> bool {
    log::warn!("is_user_admin() is only supported on Windows");
    false
}

/// Check if a registry value exists
///
/// Equivalent to C# Utils.RegistryValueExists
#[cfg(target_os = "windows")]
pub fn registry_value_exists(hive_str: &str, key: &str, value: &str) -> Result<bool> {
    use crate::ffi::windows_registry::{Hive, RegKey};

    let hive = Hive::from_str(hive_str)
        .ok_or_else(|| PersistError::Registry(format!("Invalid registry hive: {}", hive_str)))?;

    match RegKey::open(hive, key, false) {
        Ok(reg_key) => Ok(reg_key.value_exists(value)),
        Err(PersistError::NotFound(_)) => Ok(false),
        Err(e) => Err(e),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn registry_value_exists(_hive: &str, _key: &str, _value: &str) -> Result<bool> {
    Err(PersistError::PlatformNotSupported {
        technique: "registry".to_string(),
        platform: "Windows".to_string(),
    })
}

/// Check if a scheduled task exists
///
/// Equivalent to C# Utils.ScheduledTaskExists
/// Stub implementation - will be completed in task scheduler iteration
#[cfg(target_os = "windows")]
pub fn scheduled_task_exists(_name: &str) -> Result<bool> {
    // Stub - will implement in task scheduler iteration
    log::warn!("scheduled_task_exists() stub called - not yet implemented");
    Ok(false)
}

#[cfg(not(target_os = "windows"))]
pub fn scheduled_task_exists(_name: &str) -> Result<bool> {
    Err(PersistError::PlatformNotSupported {
        technique: "schtask".to_string(),
        platform: "Windows".to_string(),
    })
}

/// Check if a Windows service exists
///
/// Equivalent to C# Utils.ServiceExists
/// Stub implementation - will be completed in service FFI iteration
#[cfg(target_os = "windows")]
pub fn service_exists(_name: &str) -> Result<bool> {
    // Stub - will implement in service FFI iteration
    log::warn!("service_exists() stub called - not yet implemented");
    Ok(false)
}

#[cfg(not(target_os = "windows"))]
pub fn service_exists(_name: &str) -> Result<bool> {
    Err(PersistError::PlatformNotSupported {
        technique: "service".to_string(),
        platform: "Windows".to_string(),
    })
}

/// Check if the current user can write to a registry key
///
/// Equivalent to C# Utils.CanWriteKey
#[cfg(target_os = "windows")]
pub fn can_write_registry_key(full_key: &str) -> Result<bool> {
    use crate::ffi::windows_registry::{Hive, RegKey};

    let parts: Vec<&str> = full_key.splitn(2, '\\').collect();
    if parts.len() != 2 {
        return Err(PersistError::Registry("Invalid key format".to_string()));
    }

    let hive = Hive::from_str(parts[0])
        .ok_or_else(|| PersistError::Registry("Invalid hive".to_string()))?;

    // Try to open with write access
    RegKey::open(hive, parts[1], true).map(|_| true)
}

#[cfg(not(target_os = "windows"))]
pub fn can_write_registry_key(_key: &str) -> Result<bool> {
    Err(PersistError::PlatformNotSupported {
        technique: "registry".to_string(),
        platform: "Windows".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_sha256_checksum() {
        // Create a temp file with known content
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test content").unwrap();
        temp_file.flush().unwrap();

        let hash = sha256_checksum(temp_file.path()).unwrap();

        // Expected SHA256 of "test content"
        let expected = "6AE8A75555209FD6C44157C0AED8016E763FF435A19CF186F76863140143FF72";
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_sha256_nonexistent_file() {
        let result = sha256_checksum("/nonexistent/file/path");
        assert!(result.is_err());
    }

    #[test]
    fn test_is_user_admin() {
        // Just ensure it doesn't panic - actual result depends on execution context
        let _ = is_user_admin();
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_platform_not_supported_errors() {
        // On non-Windows, these should return platform errors
        assert!(registry_value_exists("HKCU", "Software", "Test").is_err());
        assert!(scheduled_task_exists("Test").is_err());
        assert!(service_exists("Test").is_err());
        assert!(can_write_registry_key("HKCU\\Software").is_err());
    }
}
