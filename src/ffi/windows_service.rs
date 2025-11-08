// Windows Service Control Manager FFI
//
// Safe Rust wrappers around Windows Service APIs for managing Windows services.
// This module provides RAII-based handles and safe abstractions over the Win32
// Service Control Manager (SCM) APIs.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

#[cfg(target_os = "windows")]
use windows::core::PCWSTR;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::BOOL;
#[cfg(target_os = "windows")]
use windows::Win32::System::Services::*;

use crate::core::error::{PersistError, Result};

/// Convert Rust string to null-terminated wide string
#[cfg(target_os = "windows")]
fn to_wide_string(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// RAII wrapper for Service Control Manager handle
#[cfg(target_os = "windows")]
pub struct ScmHandle(SC_HANDLE);

#[cfg(target_os = "windows")]
impl ScmHandle {
    /// Open connection to the Service Control Manager
    pub fn open(desired_access: u32) -> Result<Self> {
        let machine_name = to_wide_string(&std::env::var("COMPUTERNAME").unwrap_or_default());

        // SAFETY: OpenSCManagerW is called with valid parameters:
        // - machine_name is a valid null-terminated wide string (local machine)
        // - database_name is null (default SERVICES_ACTIVE_DATABASE)
        // - desired_access specifies requested permissions
        let handle = unsafe {
            OpenSCManagerW(
                PCWSTR(machine_name.as_ptr()),
                PCWSTR::null(),
                desired_access,
            )
        };

        if handle.is_invalid() {
            return Err(PersistError::WindowsApi(
                "Failed to open Service Control Manager".to_string(),
            ));
        }

        Ok(Self(handle))
    }

    /// Get the raw handle
    pub fn as_raw(&self) -> SC_HANDLE {
        self.0
    }
}

#[cfg(target_os = "windows")]
impl Drop for ScmHandle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            // SAFETY: CloseServiceHandle is called with a valid handle that we own
            unsafe {
                let _ = CloseServiceHandle(self.0);
            }
        }
    }
}

/// RAII wrapper for Service handle
#[cfg(target_os = "windows")]
pub struct ServiceHandle(SC_HANDLE);

#[cfg(target_os = "windows")]
impl ServiceHandle {
    /// Create a new Windows service
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        scm: &ScmHandle,
        service_name: &str,
        display_name: &str,
        binary_path: &str,
    ) -> Result<Self> {
        let service_name_wide = to_wide_string(service_name);
        let display_name_wide = to_wide_string(display_name);
        let binary_path_wide = to_wide_string(binary_path);

        // SAFETY: CreateServiceW is called with valid parameters:
        // - scm handle is valid and has CREATE_SERVICE permission
        // - all string parameters are valid null-terminated wide strings
        // - service configuration is valid (auto-start, own process, ignore errors)
        let handle = unsafe {
            CreateServiceW(
                scm.as_raw(),
                PCWSTR(service_name_wide.as_ptr()),
                PCWSTR(display_name_wide.as_ptr()),
                SERVICE_ALL_ACCESS.0,
                SERVICE_WIN32_OWN_PROCESS,
                SERVICE_AUTO_START,
                SERVICE_ERROR_IGNORE,
                PCWSTR(binary_path_wide.as_ptr()),
                PCWSTR::null(),
                None,
                PCWSTR::null(),
                PCWSTR::null(),
                PCWSTR::null(),
            )
        };

        if handle.is_invalid() {
            return Err(PersistError::WindowsApi(format!(
                "Failed to create service '{}'",
                service_name
            )));
        }

        Ok(Self(handle))
    }

    /// Open an existing Windows service
    pub fn open(scm: &ScmHandle, service_name: &str, desired_access: u32) -> Result<Self> {
        let service_name_wide = to_wide_string(service_name);

        // SAFETY: OpenServiceW is called with valid parameters:
        // - scm handle is valid
        // - service_name is a valid null-terminated wide string
        // - desired_access specifies requested permissions
        let handle = unsafe {
            OpenServiceW(
                scm.as_raw(),
                PCWSTR(service_name_wide.as_ptr()),
                desired_access,
            )
        };

        if handle.is_invalid() {
            return Err(PersistError::WindowsApi(format!(
                "Failed to open service '{}'",
                service_name
            )));
        }

        Ok(Self(handle))
    }

    /// Delete the service
    pub fn delete(&self) -> Result<()> {
        // SAFETY: DeleteService is called with a valid service handle
        let result = unsafe { DeleteService(self.0) };

        if result == BOOL(0) {
            return Err(PersistError::WindowsApi(
                "Failed to delete service".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(target_os = "windows")]
impl Drop for ServiceHandle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            // SAFETY: CloseServiceHandle is called with a valid handle that we own
            unsafe {
                let _ = CloseServiceHandle(self.0);
            }
        }
    }
}

/// Check if a service exists by name
#[cfg(target_os = "windows")]
pub fn service_exists(service_name: &str) -> bool {
    let scm = match ScmHandle::open(SC_MANAGER_ENUMERATE_SERVICE.0) {
        Ok(h) => h,
        Err(_) => return false,
    };

    ServiceHandle::open(&scm, service_name, SERVICE_QUERY_STATUS.0).is_ok()
}

/// Platform-specific stub for non-Windows platforms
#[cfg(not(target_os = "windows"))]
pub fn service_exists(_service_name: &str) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_wide_string() {
        #[cfg(target_os = "windows")]
        {
            let result = to_wide_string("test");
            assert_eq!(result.len(), 5); // "test" + null terminator
            assert_eq!(result[4], 0); // null terminator
        }
    }

    #[test]
    fn test_service_exists_nonexistent() {
        // Test with a service name that definitely doesn't exist
        assert!(!service_exists("NonExistentServiceName_12345_xyz"));
    }
}
