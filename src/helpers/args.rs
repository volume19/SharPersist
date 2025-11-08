//! Command-line argument parsing

use crate::core::{
    config::{Method, PersistConfig, Technique},
    error::Result,
};
use clap::Parser;

/// SharPersist - Windows Persistence Toolkit
#[derive(Parser, Debug)]
#[command(
    name = "sharpersist",
    version = "0.1.0",
    author = "Rust Port Contributors (Original: Brett Hawkins @h4wkst3r)",
    about = "Windows persistence toolkit for authorized security testing",
    long_about = "Windows persistence toolkit written in Rust. For authorized penetration testing and red team operations only."
)]
pub struct CliArgs {
    /// Persistence technique (keepass, reg, schtaskbackdoor, startupfolder, tortoisesvn, service, schtask)
    #[arg(short = 't', long = "technique", value_name = "TECHNIQUE")]
    pub technique: String,

    /// Method (add, remove, check, list)
    #[arg(short = 'm', long = "method", value_name = "METHOD")]
    pub method: String,

    /// Command to execute
    #[arg(short = 'c', long = "command", value_name = "COMMAND")]
    pub command: Option<String>,

    /// Arguments to command
    #[arg(short = 'a', long = "args", value_name = "ARGS")]
    pub args: Option<String>,

    /// File path (for KeePass config, startup folder LNK, etc.)
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    pub file: Option<String>,

    /// Registry key (hklmrun, hklmrunonce, hklmrunonceex, hkcurun, hkcurunonce, logonscript, stickynotes, userinit)
    #[arg(short = 'k', long = "key", value_name = "KEY")]
    pub key: Option<String>,

    /// Registry value name
    #[arg(short = 'v', long = "value", value_name = "VALUE")]
    pub value: Option<String>,

    /// Service or scheduled task name
    #[arg(short = 'n', long = "name", value_name = "NAME")]
    pub name: Option<String>,

    /// Optional add-ons (env, hourly, daily, logon)
    #[arg(short = 'o', long = "option", value_name = "OPTION")]
    pub option: Option<String>,
}

impl CliArgs {
    /// Parse from command-line arguments
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Convert CLI arguments to PersistConfig
    pub fn into_config(self) -> Result<PersistConfig> {
        let technique = Technique::from_str(&self.technique)?;
        let method = Method::from_str(&self.method)?;

        let config = PersistConfig {
            technique,
            method,
            command: self.command,
            command_arg: self.args,
            file_path: self.file,
            registry_key: self.key,
            registry_value: self.value,
            name: self.name,
            option: self.option,
        };

        // Validate configuration
        config.validate()?;

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_registry_add() {
        let args = CliArgs {
            technique: "reg".to_string(),
            method: "add".to_string(),
            command: Some("cmd.exe".to_string()),
            args: Some("/c calc.exe".to_string()),
            key: Some("hkcurun".to_string()),
            value: Some("TestValue".to_string()),
            file: None,
            name: None,
            option: None,
        };

        let config = args.into_config().unwrap();
        assert_eq!(config.technique, Technique::Registry);
        assert_eq!(config.method, Method::Add);
        assert_eq!(config.command, Some("cmd.exe".to_string()));
    }

    #[test]
    fn test_parse_service_add() {
        let args = CliArgs {
            technique: "service".to_string(),
            method: "add".to_string(),
            command: Some("cmd.exe".to_string()),
            args: Some("/c whoami".to_string()),
            name: Some("TestService".to_string()),
            key: None,
            value: None,
            file: None,
            option: None,
        };

        let config = args.into_config().unwrap();
        assert_eq!(config.technique, Technique::Service);
        assert_eq!(config.name, Some("TestService".to_string()));
    }

    #[test]
    fn test_invalid_technique() {
        let args = CliArgs {
            technique: "invalid".to_string(),
            method: "add".to_string(),
            command: None,
            args: None,
            key: None,
            value: None,
            file: None,
            name: None,
            option: None,
        };

        assert!(args.into_config().is_err());
    }
}
