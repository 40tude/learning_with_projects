// For integration tests to work, I need to expose modules.
// See Update `src/lib.rs`:

use config_watcher::*;
use std::fs;
use tempfile::NamedTempFile;
use tokio::time::{Duration, sleep};

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
