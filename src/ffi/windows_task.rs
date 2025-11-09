// Windows Task Scheduler COM Wrapper
//
// Safe Rust wrappers around Windows Task Scheduler 2.0 COM APIs.
// Provides functionality for creating, modifying, and querying scheduled tasks.

use crate::core::error::{PersistError, Result};
use std::process::Command;

/// Represents a scheduled task trigger type
#[derive(Debug, Clone, Copy)]
pub enum TriggerType {
    Daily,
    Hourly,
    Logon,
}

impl TriggerType {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "daily" => Ok(TriggerType::Daily),
            "hourly" => Ok(TriggerType::Hourly),
            "logon" => Ok(TriggerType::Logon),
            _ => Err(PersistError::InvalidInput(format!(
                "Invalid trigger type: {}",
                s
            ))),
        }
    }
}

/// Check if a scheduled task exists by name
pub fn task_exists(task_name: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        // Use schtasks /Query to check if task exists
        if let Ok(output) = Command::new("schtasks")
            .arg("/Query")
            .arg("/TN")
            .arg(task_name)
            .arg("/FO")
            .arg("LIST")
            .output()
        {
            return output.status.success();
        }
    }
    false
}

/// Create a scheduled task using schtasks.exe
#[cfg(target_os = "windows")]
pub fn create_task(task_name: &str, command: &str, trigger_type: TriggerType) -> Result<()> {
    let mut cmd = Command::new("schtasks");
    cmd.arg("/Create")
        .arg("/TN")
        .arg(task_name)
        .arg("/TR")
        .arg(command)
        .arg("/F"); // Force creation, overwrite if exists

    // Set trigger based on type
    match trigger_type {
        TriggerType::Daily => {
            cmd.arg("/SC").arg("DAILY").arg("/ST").arg("10:00");
        }
        TriggerType::Hourly => {
            cmd.arg("/SC").arg("HOURLY").arg("/MO").arg("1");
        }
        TriggerType::Logon => {
            cmd.arg("/SC").arg("ONLOGON").arg("/RU").arg("SYSTEM");
        }
    }

    let output = cmd.output()?;

    if output.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(PersistError::OperationFailed(format!(
            "Failed to create scheduled task: {}",
            error
        )))
    }
}

/// Delete a scheduled task
#[cfg(target_os = "windows")]
pub fn delete_task(task_name: &str) -> Result<()> {
    let output = Command::new("schtasks")
        .arg("/Delete")
        .arg("/TN")
        .arg(task_name)
        .arg("/F")
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(PersistError::OperationFailed(format!(
            "Failed to delete scheduled task: {}",
            error
        )))
    }
}

/// Export a scheduled task to XML
#[cfg(target_os = "windows")]
pub fn export_task_xml(task_name: &str) -> Result<String> {
    let output = Command::new("schtasks")
        .arg("/Query")
        .arg("/TN")
        .arg(task_name)
        .arg("/XML")
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(PersistError::OperationFailed(format!(
            "Failed to export task '{}'",
            task_name
        )))
    }
}

/// Import a scheduled task from XML
#[cfg(target_os = "windows")]
pub fn import_task_xml(task_name: &str, xml_content: &str) -> Result<()> {
    use std::fs;
    use std::io::Write;

    // Create temporary XML file
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("{}_temp.xml", task_name));

    // Write XML to temp file
    let mut file = fs::File::create(&temp_file)?;
    file.write_all(xml_content.as_bytes())?;
    drop(file);

    // Import using schtasks
    let output = Command::new("schtasks")
        .arg("/Create")
        .arg("/TN")
        .arg(task_name)
        .arg("/XML")
        .arg(&temp_file)
        .arg("/F")
        .output()?;

    // Clean up temp file
    let _ = fs::remove_file(&temp_file);

    if output.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(PersistError::OperationFailed(format!(
            "Failed to import task: {}",
            error
        )))
    }
}

/// List all scheduled tasks
#[cfg(target_os = "windows")]
pub fn list_all_tasks() -> Result<Vec<String>> {
    let output = Command::new("schtasks")
        .arg("/Query")
        .arg("/FO")
        .arg("LIST")
        .output()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let tasks: Vec<String> = stdout
            .lines()
            .filter(|line| line.starts_with("TaskName:"))
            .map(|line| line.trim_start_matches("TaskName:").trim().to_string())
            .filter(|name| !name.starts_with("\\Microsoft\\")) // Filter out system tasks
            .collect();

        Ok(tasks)
    } else {
        Err(PersistError::OperationFailed(
            "Failed to list scheduled tasks".to_string(),
        ))
    }
}

/// Get detailed information about a task
#[cfg(target_os = "windows")]
pub fn get_task_info(task_name: &str) -> Result<String> {
    let output = Command::new("schtasks")
        .arg("/Query")
        .arg("/TN")
        .arg(task_name)
        .arg("/FO")
        .arg("LIST")
        .arg("/V")
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(PersistError::NotFound(format!(
            "Task '{}' not found",
            task_name
        )))
    }
}

/// Platform stubs for non-Windows
#[cfg(not(target_os = "windows"))]
pub fn create_task(_task_name: &str, _command: &str, _trigger_type: TriggerType) -> Result<()> {
    Err(PersistError::PlatformNotSupported {
        technique: "ScheduledTask".to_string(),
        platform: "Windows".to_string(),
    })
}

#[cfg(not(target_os = "windows"))]
pub fn delete_task(_task_name: &str) -> Result<()> {
    Err(PersistError::PlatformNotSupported {
        technique: "ScheduledTask".to_string(),
        platform: "Windows".to_string(),
    })
}

#[cfg(not(target_os = "windows"))]
pub fn export_task_xml(_task_name: &str) -> Result<String> {
    Err(PersistError::PlatformNotSupported {
        technique: "ScheduledTask".to_string(),
        platform: "Windows".to_string(),
    })
}

#[cfg(not(target_os = "windows"))]
pub fn import_task_xml(_task_name: &str, _xml_content: &str) -> Result<()> {
    Err(PersistError::PlatformNotSupported {
        technique: "ScheduledTask".to_string(),
        platform: "Windows".to_string(),
    })
}

#[cfg(not(target_os = "windows"))]
pub fn list_all_tasks() -> Result<Vec<String>> {
    Err(PersistError::PlatformNotSupported {
        technique: "ScheduledTask".to_string(),
        platform: "Windows".to_string(),
    })
}

#[cfg(not(target_os = "windows"))]
pub fn get_task_info(_task_name: &str) -> Result<String> {
    Err(PersistError::PlatformNotSupported {
        technique: "ScheduledTask".to_string(),
        platform: "Windows".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger_type_from_str() {
        assert!(TriggerType::from_str("daily").is_ok());
        assert!(TriggerType::from_str("hourly").is_ok());
        assert!(TriggerType::from_str("logon").is_ok());
        assert!(TriggerType::from_str("invalid").is_err());
    }

    #[test]
    fn test_task_exists_nonexistent() {
        assert!(!task_exists("NonExistentTask_xyz123"));
    }
}
