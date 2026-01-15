/******************************************************************************

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

******************************************************************************/

use crate::config::AppConfig;
use crate::error::{ConfigError, Result};
use anyhow::Context;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::fs;
use tokio::time::{Duration, interval};

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
        let config: AppConfig =
            serde_json::from_str(&contents).context("Failed to parse JSON configuration")?;

        // Validate business rules
        config
            .validate()
            .context("Configuration validation failed")?;

        Ok(config)
    }

    /// Gets the last modified timestamp of the file
    async fn get_modified_time(&self) -> Result<SystemTime> {
        let metadata =
            fs::metadata(&self.file_path)
                .await
                .map_err(|e| ConfigError::MetadataError {
                    path: self.file_path.clone(),
                    source: e,
                })?;

        metadata.modified().map_err(|e| ConfigError::MetadataError {
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
        println!(
            "👀 Watching configuration file: {}",
            self.file_path.display()
        );
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
            println!(
                "   Server: {}:{} (SSL: {})",
                server.host, server.port, server.enable_ssl
            );
        }

        if let Some(ref db) = config.database {
            println!(
                "   Database: pool_size={}, timeout={}s",
                db.pool_size, db.timeout_seconds
            );
        }

        if !config.features.is_empty() {
            println!(
                "   Features: {} enabled",
                config.features.iter().filter(|&(_, v)| *v).count()
            );
        }
        println!();
    }
}
