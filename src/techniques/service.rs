// Windows Service Persistence
//
// Creates, removes, checks, and lists Windows services for persistence.
// This technique installs a malicious service that runs automatically at system startup.
//
// MITRE ATT&CK Technique: T1543.003 - Create or Modify System Process: Windows Service
// https://attack.mitre.org/techniques/T1543/003/

use crate::core::config::{Method, PersistConfig};
use crate::core::error::{PersistError, Result};
use crate::helpers::utils;
use log::{info, warn};

#[cfg(target_os = "windows")]
use crate::ffi::windows_service;
#[cfg(target_os = "windows")]
use windows::Win32::System::Services::*;

/// Execute service persistence operation based on configuration
pub fn execute(config: &PersistConfig) -> Result<()> {
    match config.method {
        Method::Add => add_persistence(config),
        Method::Remove => remove_persistence(config),
        Method::Check => check_persistence(config).map(|r| print_check_result(&r)),
        Method::List => list_persistence(config),
    }
}

/// Add service persistence
#[cfg(target_os = "windows")]
fn add_persistence(config: &PersistConfig) -> Result<()> {
    let command = config
        .command
        .as_ref()
        .ok_or_else(|| PersistError::MissingParameter("command".to_string()))?;

    let service_name = config
        .name
        .as_ref()
        .ok_or_else(|| PersistError::MissingParameter("name".to_string()))?;

    info!("Adding service persistence");
    info!("Command: {}", command);
    info!("Service Name: {}", service_name);

    // Check if service already exists
    if windows_service::service_exists(service_name) {
        return Err(PersistError::AlreadyExists(format!(
            "Service '{}' already exists",
            service_name
        )));
    }

    // Build full command with arguments
    let full_command = if let Some(args) = &config.command_arg {
        format!("{} {}", command, args)
    } else {
        command.clone()
    };

    // Open Service Control Manager
    let scm = windows_service::ScmHandle::open(SC_MANAGER_CREATE_SERVICE.0)?;

    // Create the service
    let _service =
        windows_service::ServiceHandle::create(&scm, service_name, service_name, &full_command)?;

    // Verify service was created
    if windows_service::service_exists(service_name) {
        println!();
        println!("[+] SUCCESS: Service persistence added");
        println!("[*] INFO: Service Name: {}", service_name);
        println!("[*] INFO: Service will start automatically on boot");
        Ok(())
    } else {
        Err(PersistError::OperationFailed(
            "Service not created successfully".to_string(),
        ))
    }
}

/// Remove service persistence
#[cfg(target_os = "windows")]
fn remove_persistence(config: &PersistConfig) -> Result<()> {
    let service_name = config
        .name
        .as_ref()
        .ok_or_else(|| PersistError::MissingParameter("name".to_string()))?;

    info!("Removing service persistence");
    info!("Service Name: {}", service_name);

    // Check if service exists
    if !windows_service::service_exists(service_name) {
        return Err(PersistError::NotFound(format!(
            "Service '{}' does not exist",
            service_name
        )));
    }

    // Open Service Control Manager
    let scm = windows_service::ScmHandle::open(SC_MANAGER_ALL_ACCESS.0)?;

    // Open the service
    let service = windows_service::ServiceHandle::open(&scm, service_name, SERVICE_ALL_ACCESS.0)?;

    // Delete the service
    service.delete()?;

    println!();
    println!("[+] SUCCESS: Service persistence removed");
    println!("[*] INFO: Service '{}' has been deleted", service_name);

    Ok(())
}

/// Check if service persistence can be added
#[cfg(target_os = "windows")]
fn check_persistence(config: &PersistConfig) -> Result<CheckResult> {
    let service_name = config.name.as_ref().map(|s| s.as_str()).unwrap_or("");

    let mut result = CheckResult::new();

    // Check if service already exists
    if !service_name.is_empty() {
        if windows_service::service_exists(service_name) {
            result.add_error(format!("Service '{}' already exists", service_name));
        } else {
            result.add_success(format!("Service '{}' does NOT exist", service_name));
        }
    }

    // Check for required parameters
    if config.command.is_none() || config.name.is_none() {
        result.add_error("Must provide both --command and --name".to_string());
    } else {
        result.add_success("Correct arguments provided".to_string());
    }

    // Check for admin privileges
    if utils::is_user_admin() {
        result.add_success("Current user has administrative privileges".to_string());
    } else {
        result.add_error(
            "Current user does NOT have administrative privileges (high integrity required)"
                .to_string(),
        );
    }

    Ok(result)
}

/// List services
#[cfg(target_os = "windows")]
fn list_persistence(config: &PersistConfig) -> Result<()> {
    use windows::core::PWSTR;
    use windows::Win32::System::Services::*;

    let service_name = config.name.as_ref();

    if let Some(name) = service_name {
        // List specific service
        println!();
        println!("[*] INFO: Listing service '{}'", name);
        println!();

        if !windows_service::service_exists(name) {
            return Err(PersistError::NotFound(format!(
                "Service '{}' does not exist",
                name
            )));
        }

        // Open SCM
        let scm = windows_service::ScmHandle::open(SC_MANAGER_ENUMERATE_SERVICE.0)?;

        // Open service
        let service = windows_service::ServiceHandle::open(&scm, name, SERVICE_QUERY_STATUS.0)?;

        println!("[*] INFO: SERVICE NAME:");
        println!("{}", name);
        println!();

        // Note: Getting full service details would require additional FFI work
        // For now, we just confirm the service exists
        println!("[*] INFO: Service exists and is queryable");
        println!();
    } else {
        // List all services
        println!();
        println!("[*] INFO: Listing all services");
        println!("[*] INFO: Note: This will list service names only");
        println!("[*] INFO: Use --name <service> to get details for a specific service");
        println!();

        // Note: Full service enumeration requires complex FFI with EnumServicesStatusExW
        // For the scope of this port, we'll indicate this feature requires --name
        warn!("Full service enumeration not yet implemented");
        println!("[*] INFO: Please specify --name <service> to query a specific service");
    }

    Ok(())
}

/// Platform stub for non-Windows
#[cfg(not(target_os = "windows"))]
fn add_persistence(_config: &PersistConfig) -> Result<()> {
    Err(PersistError::PlatformNotSupported {
        technique: "Service".to_string(),
        platform: "Windows".to_string(),
    })
}

/// Platform stub for non-Windows
#[cfg(not(target_os = "windows"))]
fn remove_persistence(_config: &PersistConfig) -> Result<()> {
    Err(PersistError::PlatformNotSupported {
        technique: "Service".to_string(),
        platform: "Windows".to_string(),
    })
}

/// Platform stub for non-Windows
#[cfg(not(target_os = "windows"))]
fn check_persistence(_config: &PersistConfig) -> Result<CheckResult> {
    Err(PersistError::PlatformNotSupported {
        technique: "Service".to_string(),
        platform: "Windows".to_string(),
    })
}

/// Platform stub for non-Windows
#[cfg(not(target_os = "windows"))]
fn list_persistence(_config: &PersistConfig) -> Result<()> {
    Err(PersistError::PlatformNotSupported {
        technique: "Service".to_string(),
        platform: "Windows".to_string(),
    })
}

/// Result of a check operation
#[derive(Debug)]
struct CheckResult {
    successes: Vec<String>,
    errors: Vec<String>,
}

impl CheckResult {
    fn new() -> Self {
        Self {
            successes: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn add_success(&mut self, msg: String) {
        self.successes.push(msg);
    }

    fn add_error(&mut self, msg: String) {
        self.errors.push(msg);
    }
}

fn print_check_result(result: &CheckResult) {
    println!();
    for success in &result.successes {
        println!("[+] SUCCESS: {}", success);
    }
    for error in &result.errors {
        println!("[-] ERROR: {}", error);
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::{Method, Technique};

    #[test]
    fn test_check_validation() {
        let config = PersistConfig {
            technique: Technique::Service,
            method: Method::Check,
            command: None,
            command_arg: None,
            file_path: None,
            registry_key: None,
            registry_value: None,
            name: None,
            option: None,
        };

        #[cfg(target_os = "windows")]
        {
            let result = check_persistence(&config);
            assert!(result.is_ok());
        }

        #[cfg(not(target_os = "windows"))]
        {
            let result = check_persistence(&config);
            assert!(result.is_err());
        }
    }
}
