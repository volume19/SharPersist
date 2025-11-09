// Scheduled Task Persistence
//
// Creates, removes, checks, and lists scheduled tasks for persistence.
// Tasks can be configured to run on different triggers (daily, hourly, logon).
//
// MITRE ATT&CK Technique: T1053.005 - Scheduled Task/Job: Scheduled Task
// https://attack.mitre.org/techniques/T1053/005/

use crate::core::config::{Method, PersistConfig};
use crate::core::error::{PersistError, Result};
use crate::ffi::windows_task;
use crate::helpers::utils;
use log::info;

/// Execute scheduled task operation based on configuration
pub fn execute(config: &PersistConfig) -> Result<()> {
    match config.method {
        Method::Add => add_persistence(config),
        Method::Remove => remove_persistence(config),
        Method::Check => check_persistence(config).map(|r| print_check_result(&r)),
        Method::List => list_persistence(config),
    }
}

/// Add scheduled task persistence
fn add_persistence(config: &PersistConfig) -> Result<()> {
    let command = config
        .command
        .as_ref()
        .ok_or_else(|| PersistError::MissingParameter("command".to_string()))?;

    let task_name = config
        .name
        .as_ref()
        .ok_or_else(|| PersistError::MissingParameter("name".to_string()))?;

    let trigger_option = config.option.as_deref().unwrap_or("daily");

    info!("Adding scheduled task persistence");
    info!("Command: {}", command);
    info!("Task Name: {}", task_name);
    info!("Trigger: {}", trigger_option);

    // Check if task already exists
    if windows_task::task_exists(task_name) {
        return Err(PersistError::AlreadyExists(format!(
            "Scheduled task '{}' already exists",
            task_name
        )));
    }

    // Build full command with arguments
    let full_command = if let Some(args) = &config.command_arg {
        format!("{} {}", command, args)
    } else {
        command.clone()
    };

    // Parse trigger type
    let trigger_type: windows_task::TriggerType = trigger_option.parse()?;

    // Create the scheduled task
    windows_task::create_task(task_name, &full_command, trigger_type)?;

    // Verify task was created
    if windows_task::task_exists(task_name) {
        println!();
        println!("[+] SUCCESS: Scheduled task added");
        println!("[*] INFO: Task Name: {}", task_name);
        println!("[*] INFO: Command: {}", full_command);
        println!("[*] INFO: Trigger: {}", trigger_option);
        println!();
        Ok(())
    } else {
        Err(PersistError::OperationFailed(
            "Scheduled task was not added successfully".to_string(),
        ))
    }
}

/// Remove scheduled task persistence
fn remove_persistence(config: &PersistConfig) -> Result<()> {
    let task_name = config
        .name
        .as_ref()
        .ok_or_else(|| PersistError::MissingParameter("name".to_string()))?;

    info!("Removing scheduled task persistence");
    info!("Task Name: {}", task_name);

    // Check if task exists
    if !windows_task::task_exists(task_name) {
        return Err(PersistError::NotFound(format!(
            "Scheduled task '{}' does not exist",
            task_name
        )));
    }

    // Delete the task
    windows_task::delete_task(task_name)?;

    // Verify deletion
    if !windows_task::task_exists(task_name) {
        println!();
        println!("[+] SUCCESS: Scheduled task removed");
        println!("[*] INFO: Task '{}' has been deleted", task_name);
        println!();
        Ok(())
    } else {
        Err(PersistError::OperationFailed(
            "Scheduled task was not removed successfully".to_string(),
        ))
    }
}

/// Check if scheduled task persistence can be added
fn check_persistence(config: &PersistConfig) -> Result<CheckResult> {
    let mut result = CheckResult::new();

    // Check if task already exists
    if let Some(task_name) = &config.name {
        if windows_task::task_exists(task_name) {
            result.add_error(format!("Scheduled task '{}' already exists", task_name));
        } else {
            result.add_success(format!("Scheduled task '{}' does NOT exist", task_name));
        }
    }

    // Check for required parameters
    if config.command.is_none() || config.name.is_none() {
        result.add_error("Must provide both --command and --name".to_string());
    } else {
        result.add_success("Correct arguments provided".to_string());
    }

    // Check for admin privileges (not strictly required for user-level tasks)
    if utils::is_user_admin() {
        result.add_success("Current user has administrative privileges".to_string());
    } else {
        result.add_warning(
            "Current user does NOT have administrative privileges (may be limited to user-level tasks)".to_string(),
        );
    }

    Ok(result)
}

/// List scheduled tasks
fn list_persistence(config: &PersistConfig) -> Result<()> {
    if let Some(task_name) = &config.name {
        // List specific task
        println!();
        println!("[*] INFO: Listing scheduled task '{}'", task_name);
        println!();

        if !windows_task::task_exists(task_name) {
            return Err(PersistError::NotFound(format!(
                "Scheduled task '{}' does not exist",
                task_name
            )));
        }

        let task_info = windows_task::get_task_info(task_name)?;
        println!("{}", task_info);
        println!();
    } else {
        // List all tasks
        println!();
        println!("[*] INFO: Listing all scheduled tasks (excluding Microsoft tasks)");
        println!();

        let tasks = windows_task::list_all_tasks()?;

        if tasks.is_empty() {
            println!("[*] INFO: No user-created scheduled tasks found");
        } else {
            println!("[*] INFO: Found {} user-created tasks:", tasks.len());
            println!();

            for task in tasks {
                println!("  - {}", task);
            }
        }

        println!();
        println!("[*] INFO: Use --name <task> to get detailed information about a specific task");
        println!();
    }

    Ok(())
}

/// Result of a check operation
#[derive(Debug)]
struct CheckResult {
    successes: Vec<String>,
    warnings: Vec<String>,
    errors: Vec<String>,
}

impl CheckResult {
    fn new() -> Self {
        Self {
            successes: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn add_success(&mut self, msg: String) {
        self.successes.push(msg);
    }

    fn add_warning(&mut self, msg: String) {
        self.warnings.push(msg);
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
    for warning in &result.warnings {
        println!("[!] WARNING: {}", warning);
    }
    for error in &result.errors {
        println!("[-] ERROR: {}", error);
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::Technique;

    #[test]
    fn test_check_validation() {
        let config = PersistConfig {
            technique: Technique::SchTask,
            method: Method::Check,
            command: None,
            command_arg: None,
            file_path: None,
            registry_key: None,
            registry_value: None,
            name: None,
            option: None,
        };

        let result = check_persistence(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_task_exists_nonexistent() {
        assert!(!windows_task::task_exists("NonExistentTask_xyz123"));
    }
}
