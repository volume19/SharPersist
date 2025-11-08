//! Safe wrapper around Windows Registry APIs
//!
//! This module provides a safe Rust interface to the Windows Registry,
//! wrapping the unsafe Win32 APIs with RAII handles and proper error handling.

use crate::core::error::{PersistError, Result};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, WIN32_ERROR};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE, REG_EXPAND_SZ,
    REG_OPTION_NON_VOLATILE, REG_SZ, REG_VALUE_TYPE,
};

/// Registry hive
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hive {
    /// HKEY_LOCAL_MACHINE
    LocalMachine,
    /// HKEY_CURRENT_USER
    CurrentUser,
}

impl Hive {
    /// Convert hive to Windows HKEY
    fn to_hkey(&self) -> HKEY {
        match self {
            Hive::LocalMachine => HKEY_LOCAL_MACHINE,
            Hive::CurrentUser => HKEY_CURRENT_USER,
        }
    }

    /// Parse hive from string (HKLM or HKCU)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "HKLM" => Some(Hive::LocalMachine),
            "HKCU" => Some(Hive::CurrentUser),
            _ => None,
        }
    }
}

/// Registry value type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegValueKind {
    /// String value (REG_SZ)
    String,
    /// Expandable string value (REG_EXPAND_SZ) - environment variables
    ExpandString,
}

impl RegValueKind {
    fn to_reg_type(&self) -> REG_VALUE_TYPE {
        match self {
            RegValueKind::String => REG_SZ,
            RegValueKind::ExpandString => REG_EXPAND_SZ,
        }
    }
}

/// RAII wrapper for Windows registry key handle
///
/// Automatically closes the registry key when dropped.
pub struct RegKey(HKEY);

impl RegKey {
    /// Open an existing registry key
    ///
    /// # Arguments
    /// * `hive` - Registry hive (HKLM or HKCU)
    /// * `subkey` - Subkey path (e.g., "Software\\Microsoft\\Windows\\CurrentVersion\\Run")
    /// * `writable` - Whether to open with write access
    ///
    /// # Errors
    /// Returns error if key doesn't exist or access is denied
    pub fn open(hive: Hive, subkey: &str, writable: bool) -> Result<Self> {
        let mut key = HKEY::default();
        let subkey_wide = to_wide_string(subkey);

        let access = if writable {
            KEY_READ | KEY_WRITE
        } else {
            KEY_READ
        };

        // SAFETY: RegOpenKeyExW is called with valid parameters:
        // - hkey is a valid predefined key
        // - subkey is a valid null-terminated wide string
        // - result is written to &mut key
        let result = unsafe {
            RegOpenKeyExW(
                hive.to_hkey(),
                PCWSTR(subkey_wide.as_ptr()),
                0,
                access,
                &mut key,
            )
        };

        if result.is_err() {
            let err = WIN32_ERROR(result.0 as u32);
            if err == ERROR_FILE_NOT_FOUND {
                return Err(PersistError::NotFound(format!(
                    "Registry key not found: {}\\{}",
                    hive_name(hive),
                    subkey
                )));
            }
            return Err(PersistError::Registry(format!(
                "Failed to open registry key: error code {}",
                result.0
            )));
        }

        Ok(RegKey(key))
    }

    /// Create or open a registry key
    ///
    /// Creates the key if it doesn't exist, opens if it does.
    pub fn create(hive: Hive, subkey: &str) -> Result<Self> {
        let mut key = HKEY::default();
        let subkey_wide = to_wide_string(subkey);

        // SAFETY: RegCreateKeyExW is called with valid parameters:
        // - hkey is a valid predefined key
        // - subkey is a valid null-terminated wide string
        // - result is written to &mut key
        let result = unsafe {
            RegCreateKeyExW(
                hive.to_hkey(),
                PCWSTR(subkey_wide.as_ptr()),
                0,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_READ | KEY_WRITE,
                None,
                &mut key,
                None,
            )
        };

        if result.is_err() {
            return Err(PersistError::Registry(format!(
                "Failed to create registry key: error code {}",
                result.0
            )));
        }

        Ok(RegKey(key))
    }

    /// Set a registry value
    ///
    /// # Arguments
    /// * `name` - Value name
    /// * `data` - String data to write
    /// * `kind` - Value type (String or ExpandString)
    pub fn set_value(&self, name: &str, data: &str, kind: RegValueKind) -> Result<()> {
        let name_wide = to_wide_string(name);
        let data_wide = to_wide_string(data);

        // Data includes null terminator
        let data_bytes = unsafe {
            std::slice::from_raw_parts(data_wide.as_ptr() as *const u8, data_wide.len() * 2)
        };

        // SAFETY: RegSetValueExW is called with valid parameters:
        // - hkey is a valid open key handle
        // - value name and data are valid null-terminated wide strings
        let result = unsafe {
            RegSetValueExW(
                self.0,
                PCWSTR(name_wide.as_ptr()),
                0,
                kind.to_reg_type(),
                Some(data_bytes),
            )
        };

        if result.is_err() {
            return Err(PersistError::Registry(format!(
                "Failed to set registry value: error code {}",
                result.0
            )));
        }

        Ok(())
    }

    /// Get a registry value as a string
    ///
    /// # Arguments
    /// * `name` - Value name
    ///
    /// # Returns
    /// The value data as a String, or error if not found
    pub fn get_value(&self, name: &str) -> Result<String> {
        let name_wide = to_wide_string(name);
        let mut data_size: u32 = 0;
        let mut data_type = REG_VALUE_TYPE::default();

        // First call to get size
        // SAFETY: RegQueryValueExW is called with null data buffer to query size
        let result = unsafe {
            RegQueryValueExW(
                self.0,
                PCWSTR(name_wide.as_ptr()),
                None,
                Some(&mut data_type),
                None,
                Some(&mut data_size),
            )
        };

        if result.is_err() {
            let err = WIN32_ERROR(result.0 as u32);
            if err == ERROR_FILE_NOT_FOUND {
                return Err(PersistError::NotFound(format!(
                    "Registry value not found: {}",
                    name
                )));
            }
            return Err(PersistError::Registry(format!(
                "Failed to query registry value size: error code {}",
                result.0
            )));
        }

        // Allocate buffer and read data
        let mut buffer = vec![0u16; (data_size as usize + 1) / 2];
        let data_bytes = unsafe {
            std::slice::from_raw_parts_mut(buffer.as_mut_ptr() as *mut u8, data_size as usize)
        };

        // SAFETY: RegQueryValueExW is called with valid buffer
        let result = unsafe {
            RegQueryValueExW(
                self.0,
                PCWSTR(name_wide.as_ptr()),
                None,
                None,
                Some(data_bytes),
                Some(&mut data_size),
            )
        };

        if result.is_err() {
            return Err(PersistError::Registry(format!(
                "Failed to read registry value: error code {}",
                result.0
            )));
        }

        // Convert from wide string to String
        Ok(from_wide_string(&buffer))
    }

    /// Delete a registry value
    ///
    /// # Arguments
    /// * `name` - Value name to delete
    pub fn delete_value(&self, name: &str) -> Result<()> {
        let name_wide = to_wide_string(name);

        // SAFETY: RegDeleteValueW is called with valid parameters
        let result = unsafe { RegDeleteValueW(self.0, PCWSTR(name_wide.as_ptr())) };

        if result.is_err() {
            let err = WIN32_ERROR(result.0 as u32);
            if err == ERROR_FILE_NOT_FOUND {
                return Err(PersistError::NotFound(format!(
                    "Registry value not found: {}",
                    name
                )));
            }
            return Err(PersistError::Registry(format!(
                "Failed to delete registry value: error code {}",
                result.0
            )));
        }

        Ok(())
    }

    /// Check if a registry value exists
    ///
    /// # Arguments
    /// * `name` - Value name to check
    ///
    /// # Returns
    /// `true` if value exists, `false` otherwise
    pub fn value_exists(&self, name: &str) -> bool {
        self.get_value(name).is_ok()
    }
}

impl Drop for RegKey {
    fn drop(&mut self) {
        // SAFETY: self.0 is a valid HKEY handle obtained from RegOpenKeyExW or RegCreateKeyExW
        // RegCloseKey is safe to call on valid handles
        if !self.0.is_invalid() {
            unsafe {
                let _ = RegCloseKey(self.0);
            }
        }
    }
}

/// Helper: Convert Rust string to null-terminated wide string
fn to_wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Helper: Convert wide string to Rust String
fn from_wide_string(wide: &[u16]) -> String {
    let end = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    String::from_utf16_lossy(&wide[..end])
}

/// Helper: Get hive display name
fn hive_name(hive: Hive) -> &'static str {
    match hive {
        Hive::LocalMachine => "HKLM",
        Hive::CurrentUser => "HKCU",
    }
}

/// Map registry key shortcut to full path and hive
///
/// Equivalent to C# Utils.getRegKeyMapping
pub fn map_registry_key(shortcut: &str) -> Option<(Hive, &'static str)> {
    match shortcut.to_lowercase().as_str() {
        "hklmrunonce" => Some((
            Hive::LocalMachine,
            "Software\\Microsoft\\Windows\\CurrentVersion\\RunOnce",
        )),
        "hklmrunonceex" => Some((
            Hive::LocalMachine,
            "Software\\Microsoft\\Windows\\CurrentVersion\\RunOnceEx",
        )),
        "hklmrun" => Some((
            Hive::LocalMachine,
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        )),
        "hkcurun" => Some((
            Hive::CurrentUser,
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        )),
        "hkcurunonce" => Some((
            Hive::CurrentUser,
            "Software\\Microsoft\\Windows\\CurrentVersion\\RunOnce",
        )),
        "logonscript" => Some((Hive::CurrentUser, "Environment")),
        "stickynotes" => Some((
            Hive::CurrentUser,
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        )),
        "userinit" => Some((
            Hive::LocalMachine,
            "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Winlogon",
        )),
        _ => None,
    }
}

/// Map registry key shortcut to predefined value name
///
/// Equivalent to C# Utils.getRegValueMapping
pub fn map_registry_value(shortcut: &str) -> Option<&'static str> {
    match shortcut.to_lowercase().as_str() {
        "logonscript" => Some("UserInitMprLogonScript"),
        "stickynotes" => Some("RESTART_STICKY_NOTES"),
        "userinit" => Some("Userinit"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hive_from_str() {
        assert_eq!(Hive::from_str("HKLM"), Some(Hive::LocalMachine));
        assert_eq!(Hive::from_str("hklm"), Some(Hive::LocalMachine));
        assert_eq!(Hive::from_str("HKCU"), Some(Hive::CurrentUser));
        assert_eq!(Hive::from_str("hkcu"), Some(Hive::CurrentUser));
        assert_eq!(Hive::from_str("invalid"), None);
    }

    #[test]
    fn test_map_registry_key() {
        let (hive, path) = map_registry_key("hkcurun").unwrap();
        assert_eq!(hive, Hive::CurrentUser);
        assert_eq!(path, "Software\\Microsoft\\Windows\\CurrentVersion\\Run");

        let (hive, path) = map_registry_key("hklmrun").unwrap();
        assert_eq!(hive, Hive::LocalMachine);
        assert_eq!(path, "Software\\Microsoft\\Windows\\CurrentVersion\\Run");

        assert!(map_registry_key("invalid").is_none());
    }

    #[test]
    fn test_map_registry_value() {
        assert_eq!(
            map_registry_value("logonscript"),
            Some("UserInitMprLogonScript")
        );
        assert_eq!(
            map_registry_value("stickynotes"),
            Some("RESTART_STICKY_NOTES")
        );
        assert_eq!(map_registry_value("userinit"), Some("Userinit"));
        assert_eq!(map_registry_value("invalid"), None);
    }

    #[test]
    fn test_wide_string_conversion() {
        let s = "test";
        let wide = to_wide_string(s);
        let back = from_wide_string(&wide);
        assert_eq!(s, back);
    }
}
