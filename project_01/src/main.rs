// cargo run -p config_watcher

/******************************************************************************

**Key Rust concepts**:
- **`#[tokio::main]`**: Macro that creates async runtime and runs main
- **`tokio::select!`**: Runs multiple futures concurrently, proceeds with first to complete
- **`signal::ctrl_c()`**: Async future that completes on Ctrl+C
- **`anyhow::Result`**: Top-level error type for applications

**Design decisions**:
- Graceful shutdown on Ctrl+C using `tokio::select!`
- Contextual error messages throughout
- Clean separation of concerns (CLI, logic, errors)

******************************************************************************/

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
    args.validate().context("Invalid command-line arguments")?;

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
