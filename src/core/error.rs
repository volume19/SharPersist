//! Error types for persistence operations

use std::io;

/// Result type for persistence operations
pub type Result<T> = std::result::Result<T, PersistError>;

/// Errors that can occur during persistence operations
#[derive(Debug, thiserror::Error)]
pub enum PersistError {
    /// I/O error (file operations, etc.)
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Invalid technique name provided
    #[error("Invalid technique: {0}")]
    InvalidTechnique(String),

    /// Invalid method provided
    #[error("Invalid method: {0}")]
    InvalidMethod(String),

    /// Required argument missing
    #[error("Missing required argument: {0}")]
    MissingArgument(String),

    /// Required parameter missing
    #[error("Missing required parameter: {0}")]
    MissingParameter(String),

    /// Invalid input provided
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Platform not supported for this operation
    #[error("Platform not supported: {technique} is only available on {platform}")]
    PlatformNotSupported { technique: String, platform: String },

    /// Permission denied (insufficient privileges)
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Registry operation failed
    #[error("Registry error: {0}")]
    Registry(String),

    /// Service operation failed
    #[error("Service error: {0}")]
    Service(String),

    /// Scheduled task operation failed
    #[error("Scheduled task error: {0}")]
    ScheduledTask(String),

    /// Resource already exists
    #[error("Resource already exists: {0}")]
    AlreadyExists(String),

    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Operation failed
    #[error("Operation failed: {0}")]
    OperationFailed(String),

    /// Windows API error
    #[error("Windows API error: {0}")]
    WindowsApi(String),

    /// Generic error with message
    #[error("{0}")]
    Other(String),
}
