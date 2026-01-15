/******************************************************************************

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

******************************************************************************/

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