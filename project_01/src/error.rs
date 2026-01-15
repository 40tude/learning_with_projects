/******************************************************************************

**Key Rust concepts**:
- **`#[error(...)]`**: Defines the Display message with formatting
- **`#[source]`**: Marks the underlying error for `Error::source()`
- **`#[from]`**: Automatically implements `From<serde_json::Error>` for easy `?` operator use
- **Type alias**: `Result<T>` is a common pattern in Rust libraries

**Design decisions**:
- Structured errors with context (file paths, reasons)
- Separate error variants for different failure modes
- Using `#[from]` for JSON errors since they're common

******************************************************************************/

use std::path::PathBuf;
use thiserror::Error;

/// Custom error types for the config watcher
///
/// Using thiserror reduces boilerplate by automatically implementing
/// std::error::Error, Display, and From conversions
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Occurs when the configuration file cannot be found
    #[error("Configuration file not found: {path}")]
    FileNotFound { path: PathBuf },

    /// Occurs when file metadata cannot be read (permissions, etc.)
    #[error("Cannot access file metadata: {path}")]
    MetadataError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Occurs when JSON parsing fails
    #[error("Invalid JSON in configuration file")]
    InvalidJson {
        #[from]
        source: serde_json::Error,
    },

    /// Occurs when the config structure doesn't match expected schema
    #[error("Configuration validation failed: {reason}")]
    ValidationFailed { reason: String },

    /// Occurs when file read operation fails
    #[error("Failed to read configuration file: {path}")]
    ReadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Result type alias for operations that return ConfigError
///
/// This is idiomatic Rust - creating type aliases for Result
/// with your error type reduces verbosity throughout the codebase
pub type Result<T> = std::result::Result<T, ConfigError>;
