## Description



## Testing

```powershell
cargo check -p config_watcher
cargo test -p config_watcher
```

## Build

 ```powershell
cargo build --release -p config_watcher
```

## Usage

```powershell
# View help
cargo run -- --help -p config_watcher

# Run with example config
cargo run -p config_watcher -- --file prj01_example_config.json --interval 2
# Open `prj01_example_config.json`
# Modify values (e.g., change version to "2.0.0")
# Save the file
# Watch the terminal detect and validate changes
```


---


**Question to Claude:**
Ok. Let’s proceed with Project 1. Guide me in building it completely from scratch in Rust, following production-quality coding practices.
Break the implementation into clear, incremental steps. For each step, explain the design decision, the Rust concepts involved, and provide idiomatic code. Use the specified crates throughout. After implementation, include unit tests and highlight common pitfalls.

# Project 1: Async Configuration File Watcher - Complete Implementation Guide

## Project Overview

We'll build `config-watcher`, a CLI tool that monitors a JSON configuration file for changes and validates its structure in real-time.

**Features**:
- Watch a JSON config file for modifications
- Validate against a defined schema
- Configurable check interval
- Graceful shutdown on Ctrl+C
- Detailed error reporting

---

## Step 1: Project Setup and Dependencies

### Create the project

```powershell
cargo new config-watcher
cd config-watcher
```

### Edit `Cargo.toml`

```toml
[package]
name = "config-watcher"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4.5", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.42", features = ["full"] }
anyhow = "1.0"
thiserror = "2.0"

[dev-dependencies]
tempfile = "3.14"
```

**Design decisions**:
- `clap` with `derive` feature for ergonomic CLI parsing
- `serde` with `derive` for automatic serialization code generation
- `tokio` with `full` features (for learning; production would be selective)
- `tempfile` for creating temporary test files

---

## Step 2: Define Custom Error Types with `thiserror`

Create `src/error.rs`:

```rust
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
```

**Key Rust concepts**:
- **`#[error(...)]`**: Defines the Display message with formatting
- **`#[source]`**: Marks the underlying error for `Error::source()`
- **`#[from]`**: Automatically implements `From<serde_json::Error>` for easy `?` operator use
- **Type alias**: `Result<T>` is a common pattern in Rust libraries

**Design decisions**:
- Structured errors with context (file paths, reasons)
- Separate error variants for different failure modes
- Using `#[from]` for JSON errors since they're common

---

## Step 3: Define Configuration Schema with `serde`

Create `src/config.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Application configuration structure
///
/// This represents the expected schema of our JSON config file.
/// Serde will handle serialization/deserialization automatically.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    /// Application name (required)
    pub app_name: String,

    /// Application version (required)
    pub version: String,

    /// Environment (development, staging, production)
    #[serde(default = "default_environment")]
    pub environment: String,

    /// Server configuration (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<ServerConfig>,

    /// Database configuration (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<DatabaseConfig>,

    /// Feature flags (optional)
    #[serde(default)]
    pub features: HashMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,

    #[serde(default = "default_true")]
    pub enable_ssl: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DatabaseConfig {
    pub connection_string: String,

    #[serde(default = "default_pool_size")]
    pub pool_size: u32,

    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

// Default value functions for serde
fn default_environment() -> String {
    "development".to_string()
}

fn default_true() -> bool {
    true
}

fn default_pool_size() -> u32 {
    10
}

fn default_timeout() -> u64 {
    30
}

impl AppConfig {
    /// Validates the configuration structure
    ///
    /// This goes beyond serde's type checking to enforce business rules
    pub fn validate(&self) -> crate::error::Result<()> {
        use crate::error::ConfigError;

        // Validate app_name is not empty
        if self.app_name.trim().is_empty() {
            return Err(ConfigError::ValidationFailed {
                reason: "app_name cannot be empty".to_string(),
            });
        }

        // Validate version format (basic semver check)
        if !self.version.contains('.') {
            return Err(ConfigError::ValidationFailed {
                reason: format!("version '{}' should follow semver format (e.g., 1.0.0)", self.version),
            });
        }

        // Validate environment values
        let valid_envs = ["development", "staging", "production"];
        if !valid_envs.contains(&self.environment.as_str()) {
            return Err(ConfigError::ValidationFailed {
                reason: format!(
                    "environment must be one of: {}",
                    valid_envs.join(", ")
                ),
            });
        }

        // Validate server config if present
        if let Some(ref server) = self.server {
            if server.host.trim().is_empty() {
                return Err(ConfigError::ValidationFailed {
                    reason: "server.host cannot be empty".to_string(),
                });
            }
            if server.port == 0 {
                return Err(ConfigError::ValidationFailed {
                    reason: "server.port must be greater than 0".to_string(),
                });
            }
        }

        // Validate database config if present
        if let Some(ref db) = self.database {
            if db.connection_string.trim().is_empty() {
                return Err(ConfigError::ValidationFailed {
                    reason: "database.connection_string cannot be empty".to_string(),
                });
            }
            if db.pool_size == 0 {
                return Err(ConfigError::ValidationFailed {
                    reason: "database.pool_size must be greater than 0".to_string(),
                });
            }
        }

        Ok(())
    }
}
```

**Key Rust concepts**:
- **`#[derive(Serialize, Deserialize)]`**: Automatic serde implementation
- **`#[serde(default)]`**: Uses Default trait or custom function if field is missing
- **`#[serde(skip_serializing_if)]`**: Omits field from output if condition is true
- **`impl` blocks**: Methods associated with the struct
- **Business validation**: Separate from type validation

**Design decisions**:
- Using `Option<T>` for optional configuration sections
- Providing sensible defaults with `#[serde(default)]`
- Validation logic separate from deserialization (business rules vs. type safety)

---

## Step 4: Implement File Watching Logic with `tokio`

Create `src/watcher.rs`:

```rust
use crate::config::AppConfig;
use crate::error::{ConfigError, Result};
use anyhow::Context;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::fs;
use tokio::time::{interval, Duration};

/// Watches a configuration file for changes and validates it
pub struct ConfigWatcher {
    file_path: PathBuf,
    check_interval: Duration,
    last_modified: Option<SystemTime>,
    last_valid_config: Option<AppConfig>,
}

impl ConfigWatcher {
    /// Creates a new ConfigWatcher instance
    ///
    /// # Arguments
    /// * `file_path` - Path to the configuration file to watch
    /// * `check_interval` - How often to check for changes (in seconds)
    pub fn new(file_path: impl AsRef<Path>, check_interval_secs: u64) -> Self {
        Self {
            file_path: file_path.as_ref().to_path_buf(),
            check_interval: Duration::from_secs(check_interval_secs),
            last_modified: None,
            last_valid_config: None,
        }
    }

    /// Reads and parses the configuration file
    ///
    /// Uses anyhow::Context to add contextual information to errors
    async fn read_config(&self) -> anyhow::Result<AppConfig> {
        // Check if file exists
        if !self.file_path.exists() {
            return Err(ConfigError::FileNotFound {
                path: self.file_path.clone(),
            }
            .into());
        }

        // Read file contents asynchronously
        let contents = fs::read_to_string(&self.file_path)
            .await
            .map_err(|e| ConfigError::ReadError {
                path: self.file_path.clone(),
                source: e,
            })
            .context("Failed to read configuration file")?;

        // Parse JSON
        let config: AppConfig = serde_json::from_str(&contents)
            .context("Failed to parse JSON configuration")?;

        // Validate business rules
        config
            .validate()
            .context("Configuration validation failed")?;

        Ok(config)
    }

    /// Gets the last modified timestamp of the file
    async fn get_modified_time(&self) -> Result<SystemTime> {
        let metadata = fs::metadata(&self.file_path)
            .await
            .map_err(|e| ConfigError::MetadataError {
                path: self.file_path.clone(),
                source: e,
            })?;

        metadata
            .modified()
            .map_err(|e| ConfigError::MetadataError {
                path: self.file_path.clone(),
                source: e,
            })
    }

    /// Checks if the file has been modified since last check
    async fn has_changed(&self) -> Result<bool> {
        let current_modified = self.get_modified_time().await?;

        Ok(match self.last_modified {
            Some(last) => current_modified > last,
            None => true, // First check always returns true
        })
    }

    /// Main watch loop - monitors file for changes
    ///
    /// This is the core async logic using tokio
    pub async fn watch(&mut self) -> anyhow::Result<()> {
        println!("👀 Watching configuration file: {}", self.file_path.display());
        println!("⏱️  Check interval: {:?}", self.check_interval);
        println!("Press Ctrl+C to stop\n");

        // Create an interval timer
        let mut ticker = interval(self.check_interval);

        // Initial load
        match self.read_config().await {
            Ok(config) => {
                println!("✅ Initial configuration loaded successfully");
                self.print_config_summary(&config);
                self.last_modified = Some(self.get_modified_time().await?);
                self.last_valid_config = Some(config);
            }
            Err(e) => {
                eprintln!("❌ Failed to load initial configuration: {:#}", e);
                eprintln!("   Waiting for valid configuration...\n");
            }
        }

        // Watch loop
        loop {
            ticker.tick().await; // Wait for next interval

            match self.has_changed().await {
                Ok(true) => {
                    println!("🔄 File change detected, reloading...");

                    match self.read_config().await {
                        Ok(config) => {
                            println!("✅ Configuration reloaded successfully");

                            // Show what changed
                            if let Some(ref last_config) = self.last_valid_config {
                                if last_config != &config {
                                    println!("📝 Configuration has been updated");
                                    self.print_config_summary(&config);
                                } else {
                                    println!("   (File modified but content unchanged)");
                                }
                            } else {
                                self.print_config_summary(&config);
                            }

                            self.last_modified = Some(self.get_modified_time().await?);
                            self.last_valid_config = Some(config);
                        }
                        Err(e) => {
                            eprintln!("❌ Configuration reload failed: {:#}", e);
                            eprintln!("   Keeping last valid configuration\n");
                        }
                    }
                }
                Ok(false) => {
                    // No changes, continue watching silently
                }
                Err(e) => {
                    eprintln!("⚠️  Error checking file: {:#}", e);
                }
            }
        }
    }

    /// Prints a summary of the configuration
    fn print_config_summary(&self, config: &AppConfig) {
        println!("   App: {} v{}", config.app_name, config.version);
        println!("   Environment: {}", config.environment);

        if let Some(ref server) = config.server {
            println!("   Server: {}:{} (SSL: {})",
                     server.host, server.port, server.enable_ssl);
        }

        if let Some(ref db) = config.database {
            println!("   Database: pool_size={}, timeout={}s",
                     db.pool_size, db.timeout_seconds);
        }

        if !config.features.is_empty() {
            println!("   Features: {} enabled",
                     config.features.iter().filter(|(_, &v)| v).count());
        }
        println!();
    }
}
```

**Key Rust concepts**:
- **`async fn`**: Asynchronous function that returns a Future
- **`.await`**: Suspends execution until Future completes
- **`tokio::time::interval`**: Creates a periodic timer
- **`tokio::fs`**: Async file system operations
- **Method chaining**: `.map_err().context()` for error transformation
- **`loop`**: Infinite loop for watching (will be cancelled by Ctrl+C)

**Design decisions**:
- Storing last modified time to detect changes efficiently
- Keeping last valid config to fall back on errors
- Using `anyhow::Context` for rich error messages
- Separating concerns: reading, parsing, validating, watching

---

## Step 5: CLI Interface with `clap`

Create `src/cli.rs`:

```rust
use clap::Parser;
use std::path::PathBuf;

/// A tool to watch and validate JSON configuration files in real-time
///
/// This CLI tool monitors a configuration file for changes and validates
/// its structure against a predefined schema. Perfect for development
/// environments where configs change frequently.
#[derive(Parser, Debug)]
#[command(name = "config-watcher")]
#[command(author = "Your Name <your.email@example.com>")]
#[command(version = "0.1.0")]
#[command(about = "Watch and validate JSON configuration files", long_about = None)]
pub struct Cli {
    /// Path to the configuration file to watch
    ///
    /// This should be a JSON file matching the expected schema
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    pub config_file: PathBuf,

    /// Check interval in seconds
    ///
    /// How frequently to check if the file has been modified
    #[arg(short = 'i', long = "interval", default_value = "2", value_name = "SECONDS")]
    pub interval: u64,

    /// Enable verbose output
    ///
    /// Shows detailed information about configuration changes
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,
}

impl Cli {
    /// Parses command-line arguments
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Validates CLI arguments
    pub fn validate(&self) -> anyhow::Result<()> {
        // Validate interval is reasonable
        if self.interval == 0 {
            anyhow::bail!("Interval must be greater than 0 seconds");
        }

        if self.interval > 3600 {
            anyhow::bail!("Interval cannot exceed 3600 seconds (1 hour)");
        }

        Ok(())
    }
}
```

**Key Rust concepts**:
- **`#[derive(Parser)]`**: Generates CLI parsing code
- **`#[command(...)]`**: Configures command metadata
- **`#[arg(...)]`**: Configures individual arguments
- **`anyhow::bail!`**: Early return with error (similar to `return Err()`)

**Design decisions**:
- Using long and short flags (`-f` and `--file`)
- Providing sensible defaults
- Validation in separate method for testability
- Comprehensive help text for user experience

---

## Step 6: Main Application with Signal Handling

Update `src/main.rs`:

```rust
mod cli;
mod config;
mod error;
mod watcher;

use anyhow::Context;
use cli::Cli;
use tokio::signal;
use watcher::ConfigWatcher;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command-line arguments
    let args = Cli::parse_args();

    // Validate arguments
    args.validate()
        .context("Invalid command-line arguments")?;

    // Create watcher instance
    let mut watcher = ConfigWatcher::new(&args.config_file, args.interval);

    // Setup graceful shutdown
    // This uses tokio::select! to race between watch loop and Ctrl+C
    tokio::select! {
        result = watcher.watch() => {
            // Watch loop ended (shouldn't happen unless error)
            result.context("Watcher error")?;
        }
        _ = signal::ctrl_c() => {
            // User pressed Ctrl+C
            println!("\n👋 Shutting down gracefully...");
        }
    }

    Ok(())
}
```

**Key Rust concepts**:
- **`#[tokio::main]`**: Macro that creates async runtime and runs main
- **`tokio::select!`**: Runs multiple futures concurrently, proceeds with first to complete
- **`signal::ctrl_c()`**: Async future that completes on Ctrl+C
- **`anyhow::Result`**: Top-level error type for applications

**Design decisions**:
- Graceful shutdown on Ctrl+C using `tokio::select!`
- Contextual error messages throughout
- Clean separation of concerns (CLI, logic, errors)

---

## Step 7: Example Configuration File

Create `example-config.json` in project root:

```json
{
  "app_name": "MyAwesomeApp",
  "version": "1.0.0",
  "environment": "development",
  "server": {
    "host": "localhost",
    "port": 8080,
    "enable_ssl": false
  },
  "database": {
    "connection_string": "postgres://localhost/mydb",
    "pool_size": 15,
    "timeout_seconds": 45
  },
  "features": {
    "enable_caching": true,
    "enable_analytics": false,
    "debug_mode": true
  }
}
```

---

## Step 8: Unit Tests

Create `src/config.rs` tests (add at the bottom of `config.rs`):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_config_deserialization() {
        let json = r#"
        {
            "app_name": "TestApp",
            "version": "1.0.0",
            "environment": "production"
        }
        "#;

        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.app_name, "TestApp");
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.environment, "production");
    }

    #[test]
    fn test_config_with_defaults() {
        let json = r#"
        {
            "app_name": "TestApp",
            "version": "1.0.0"
        }
        "#;

        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.environment, "development"); // default value
        assert!(config.features.is_empty()); // default empty HashMap
    }

    #[test]
    fn test_config_validation_empty_app_name() {
        let config = AppConfig {
            app_name: "".to_string(),
            version: "1.0.0".to_string(),
            environment: "development".to_string(),
            server: None,
            database: None,
            features: HashMap::new(),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_version() {
        let config = AppConfig {
            app_name: "TestApp".to_string(),
            version: "1".to_string(), // No dot, invalid semver
            environment: "development".to_string(),
            server: None,
            database: None,
            features: HashMap::new(),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_environment() {
        let config = AppConfig {
            app_name: "TestApp".to_string(),
            version: "1.0.0".to_string(),
            environment: "invalid".to_string(),
            server: None,
            database: None,
            features: HashMap::new(),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_server_config_validation() {
        let config = AppConfig {
            app_name: "TestApp".to_string(),
            version: "1.0.0".to_string(),
            environment: "development".to_string(),
            server: Some(ServerConfig {
                host: "".to_string(), // Empty host should fail
                port: 8080,
                enable_ssl: false,
            }),
            database: None,
            features: HashMap::new(),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_database_config_validation() {
        let config = AppConfig {
            app_name: "TestApp".to_string(),
            version: "1.0.0".to_string(),
            environment: "development".to_string(),
            server: None,
            database: Some(DatabaseConfig {
                connection_string: "postgres://localhost/db".to_string(),
                pool_size: 0, // Zero pool size should fail
                timeout_seconds: 30,
            }),
            features: HashMap::new(),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_valid_complete_config() {
        let config = AppConfig {
            app_name: "TestApp".to_string(),
            version: "1.0.0".to_string(),
            environment: "production".to_string(),
            server: Some(ServerConfig {
                host: "localhost".to_string(),
                port: 8080,
                enable_ssl: true,
            }),
            database: Some(DatabaseConfig {
                connection_string: "postgres://localhost/db".to_string(),
                pool_size: 10,
                timeout_seconds: 30,
            }),
            features: HashMap::from([
                ("feature1".to_string(), true),
                ("feature2".to_string(), false),
            ]),
        };

        assert!(config.validate().is_ok());
    }
}
```

Add integration tests in `tests/integration_test.rs`:

```rust
use config_watcher::*;
use std::fs;
use tempfile::NamedTempFile;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_watcher_detects_changes() {
    // Create a temporary config file
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();

    // Write initial config
    let initial_config = r#"
    {
        "app_name": "TestApp",
        "version": "1.0.0",
        "environment": "development"
    }
    "#;
    fs::write(path, initial_config).unwrap();

    // Create watcher with short interval
    let mut watcher = watcher::ConfigWatcher::new(path, 1);

    // Spawn watcher in background
    let watcher_handle = tokio::spawn(async move {
        let _ = watcher.watch().await;
    });

    // Wait a bit
    sleep(Duration::from_secs(2)).await;

    // Modify file
    let updated_config = r#"
    {
        "app_name": "TestApp",
        "version": "2.0.0",
        "environment": "production"
    }
    "#;
    fs::write(path, updated_config).unwrap();

    // Wait for detection
    sleep(Duration::from_secs(2)).await;

    // Cleanup
    watcher_handle.abort();
}

#[tokio::test]
async fn test_watcher_handles_invalid_json() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();

    // Write invalid JSON
    fs::write(path, "{ invalid json }").unwrap();

    let mut watcher = watcher::ConfigWatcher::new(path, 1);

    // Watcher should handle the error gracefully
    // We'll just verify it doesn't panic
    let watcher_handle = tokio::spawn(async move {
        let _ = watcher.watch().await;
    });

    sleep(Duration::from_secs(2)).await;
    watcher_handle.abort();
}
```

**Note**: For integration tests to work, you need to expose modules. Update `src/lib.rs`:

```rust
pub mod cli;
pub mod config;
pub mod error;
pub mod watcher;
```

**Running tests**:

```powershell
cargo test
```

---

## Step 9: Running the Application

```powershell
# Build the project
cargo build --release

# Run with example config
cargo run -- --file example-config.json --interval 2

# Or using the compiled binary
./target/release/config-watcher.exe --file example-config.json --interval 2

# View help
cargo run -- --help
```

**Try it out**:
1. Run the watcher
2. Open `example-config.json` in another editor
3. Modify values (e.g., change version to "2.0.0")
4. Save the file
5. Watch the terminal detect and validate changes

---

## Common Pitfalls & Solutions

### 1. **Pitfall: Not handling errors at async boundaries**
```rust
// ❌ Bad: Unwrap in async context
async fn bad_read() {
    let content = fs::read_to_string("file.json").await.unwrap();
}

// ✅ Good: Propagate errors with ?
async fn good_read() -> Result<String> {
    let content = fs::read_to_string("file.json").await?;
    Ok(content)
}
```

### 2. **Pitfall: Blocking operations in async functions**
```rust
// ❌ Bad: Blocking file I/O
async fn bad() {
    let content = std::fs::read_to_string("file.json").unwrap();
}

// ✅ Good: Async file I/O
async fn good() -> Result<String> {
    let content = tokio::fs::read_to_string("file.json").await?;
    Ok(content)
}
```

### 3. **Pitfall: Not providing context with anyhow**
```rust
// ❌ Bad: Generic error messages
let config: AppConfig = serde_json::from_str(&contents)?;

// ✅ Good: Contextual errors
let config: AppConfig = serde_json::from_str(&contents)
    .context(format!("Failed to parse config from {}", path.display()))?;
```

### 4. **Pitfall: Using unwrap() in production code**
```rust
// ❌ Bad: Can panic
let config = serde_json::from_str(&contents).unwrap();

// ✅ Good: Handle errors gracefully
let config = serde_json::from_str(&contents)
    .map_err(|e| ConfigError::InvalidJson { source: e })?;
```

### 5. **Pitfall: Not validating business rules**
```rust
// ❌ Bad: Only type validation
#[derive(Deserialize)]
struct Config {
    port: u16,
}

// ✅ Good: Type + business validation
impl Config {
    fn validate(&self) -> Result<()> {
        if self.port < 1024 {
            return Err(ConfigError::ValidationFailed {
                reason: "Port must be >= 1024".into()
            });
        }
        Ok(())
    }
}
```

### 6. **Pitfall: Not using tokio::select! for cancellation**
```rust
// ❌ Bad: No graceful shutdown
#[tokio::main]
async fn main() {
    watcher.watch().await.unwrap();
}

// ✅ Good: Graceful shutdown on Ctrl+C
#[tokio::main]
async fn main() {
    tokio::select! {
        _ = watcher.watch() => {}
        _ = signal::ctrl_c() => {
            println!("Shutting down...");
        }
    }
}
```

### 7. **Pitfall: Forgetting #[serde(default)] for optional fields**
```rust
// ❌ Bad: Missing fields cause deserialization errors
#[derive(Deserialize)]
struct Config {
    optional_field: String,
}

// ✅ Good: Provide defaults
#[derive(Deserialize)]
struct Config {
    #[serde(default)]
    optional_field: String,
}
```

---

## Production Improvements

For a production system, consider adding:

1. **Structured logging** with `tracing` instead of `println!`
2. **Configuration reload hooks** (callbacks when config changes)
3. **Multiple file watching** (watch a directory)
4. **Diff reporting** (show exactly what changed)
5. **Metrics** (reload count, error rate)
6. **File locking** (handle concurrent writes)
7. **Debouncing** (avoid rapid reloads)
8. **Backup configuration** (save last N valid configs)

---

## Summary

You've built a production-quality config watcher demonstrating:

- **clap**: Ergonomic CLI with validation
- **serde**: Type-safe deserialization with custom validation
- **tokio**: Async I/O and interval-based watching
- **anyhow**: Rich contextual error handling
- **thiserror**: Structured error types

**Key takeaways**:
- Separate concerns: CLI, business logic, errors
- Use `async`/`await` for I/O-bound operations
- Provide context with every error
- Validate beyond type safety
- Test both happy and error paths
- Handle signals for graceful shutdown

This project establishes patterns you'll use in all Rust applications!