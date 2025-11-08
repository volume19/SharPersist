//! Configuration types for persistence operations

#![allow(clippy::should_implement_trait)]

use crate::core::error::{PersistError, Result};
use std::fmt;

/// Persistence operation method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// Add persistence
    Add,
    /// Remove persistence
    Remove,
    /// Dry-run check (validate without modifying)
    Check,
    /// List existing persistence entries
    List,
}

impl Method {
    /// Parse method from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "add" => Ok(Method::Add),
            "remove" => Ok(Method::Remove),
            "check" => Ok(Method::Check),
            "list" => Ok(Method::List),
            _ => Err(PersistError::InvalidMethod(s.to_string())),
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Method::Add => write!(f, "add"),
            Method::Remove => write!(f, "remove"),
            Method::Check => write!(f, "check"),
            Method::List => write!(f, "list"),
        }
    }
}

/// Persistence technique type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Technique {
    /// KeePass config backdoor
    KeePass,
    /// Registry persistence
    Registry,
    /// Scheduled task backdoor
    SchTaskBackdoor,
    /// Startup folder LNK file
    StartupFolder,
    /// TortoiseSVN hook scripts
    TortoiseSVN,
    /// Windows service
    Service,
    /// Scheduled task
    SchTask,
}

impl Technique {
    /// Parse technique from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "keepass" => Ok(Technique::KeePass),
            "reg" => Ok(Technique::Registry),
            "schtaskbackdoor" => Ok(Technique::SchTaskBackdoor),
            "startupfolder" => Ok(Technique::StartupFolder),
            "tortoisesvn" => Ok(Technique::TortoiseSVN),
            "service" => Ok(Technique::Service),
            "schtask" => Ok(Technique::SchTask),
            _ => Err(PersistError::InvalidTechnique(s.to_string())),
        }
    }

    /// Get technique name as string
    pub fn as_str(&self) -> &'static str {
        match self {
            Technique::KeePass => "keepass",
            Technique::Registry => "reg",
            Technique::SchTaskBackdoor => "schtaskbackdoor",
            Technique::StartupFolder => "startupfolder",
            Technique::TortoiseSVN => "tortoisesvn",
            Technique::Service => "service",
            Technique::SchTask => "schtask",
        }
    }
}

impl fmt::Display for Technique {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Configuration for a persistence operation
///
/// This struct replaces the C# `Persistence` base class.
/// All fields are owned to avoid lifetime complexity.
#[derive(Debug, Clone)]
pub struct PersistConfig {
    /// Persistence technique to use
    pub technique: Technique,

    /// Method to execute (add, remove, check, list)
    pub method: Method,

    /// Command to execute (for add operations)
    pub command: Option<String>,

    /// Arguments to the command
    pub command_arg: Option<String>,

    /// File path (for KeePass config, startup folder LNK, etc.)
    pub file_path: Option<String>,

    /// Registry key (for registry techniques)
    pub registry_key: Option<String>,

    /// Registry value name (for registry techniques)
    pub registry_value: Option<String>,

    /// Name of service or scheduled task
    pub name: Option<String>,

    /// Optional add-on (env, hourly, daily, logon)
    pub option: Option<String>,
}

impl PersistConfig {
    /// Validate configuration has required fields for the technique/method
    pub fn validate(&self) -> Result<()> {
        // Check technique-specific required fields
        match self.technique {
            Technique::Registry => {
                if self.method == Method::Add || self.method == Method::Check {
                    if self.command.is_none() {
                        return Err(PersistError::MissingArgument("command".to_string()));
                    }
                    if self.registry_key.is_none() {
                        return Err(PersistError::MissingArgument("registry_key".to_string()));
                    }
                }
                if (self.method == Method::Remove || self.method == Method::List)
                    && self.registry_key.is_none()
                {
                    return Err(PersistError::MissingArgument("registry_key".to_string()));
                }
            }
            Technique::Service | Technique::SchTask | Technique::SchTaskBackdoor => {
                if self.method == Method::Add && self.command.is_none() {
                    return Err(PersistError::MissingArgument("command".to_string()));
                }
                if self.name.is_none() && self.method != Method::List {
                    return Err(PersistError::MissingArgument("name".to_string()));
                }
            }
            Technique::KeePass | Technique::StartupFolder => {
                if self.method == Method::Add && self.command.is_none() {
                    return Err(PersistError::MissingArgument("command".to_string()));
                }
                if self.file_path.is_none() && self.method != Method::List {
                    return Err(PersistError::MissingArgument("file_path".to_string()));
                }
            }
            Technique::TortoiseSVN => {
                if self.method == Method::Add && self.command.is_none() {
                    return Err(PersistError::MissingArgument("command".to_string()));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_from_str() {
        assert_eq!(Method::from_str("add").unwrap(), Method::Add);
        assert_eq!(Method::from_str("ADD").unwrap(), Method::Add);
        assert_eq!(Method::from_str("remove").unwrap(), Method::Remove);
        assert_eq!(Method::from_str("check").unwrap(), Method::Check);
        assert_eq!(Method::from_str("list").unwrap(), Method::List);
        assert!(Method::from_str("invalid").is_err());
    }

    #[test]
    fn test_technique_from_str() {
        assert_eq!(Technique::from_str("reg").unwrap(), Technique::Registry);
        assert_eq!(Technique::from_str("REG").unwrap(), Technique::Registry);
        assert_eq!(Technique::from_str("service").unwrap(), Technique::Service);
        assert_eq!(Technique::from_str("keepass").unwrap(), Technique::KeePass);
        assert!(Technique::from_str("invalid").is_err());
    }

    #[test]
    fn test_config_validation() {
        // Valid registry add config
        let config = PersistConfig {
            technique: Technique::Registry,
            method: Method::Add,
            command: Some("cmd.exe".to_string()),
            command_arg: None,
            registry_key: Some("hkcurun".to_string()),
            registry_value: Some("Test".to_string()),
            file_path: None,
            name: None,
            option: None,
        };
        assert!(config.validate().is_ok());

        // Invalid - missing command
        let config = PersistConfig {
            technique: Technique::Registry,
            method: Method::Add,
            command: None,
            command_arg: None,
            registry_key: Some("hkcurun".to_string()),
            registry_value: Some("Test".to_string()),
            file_path: None,
            name: None,
            option: None,
        };
        assert!(config.validate().is_err());
    }
}
