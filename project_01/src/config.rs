/******************************************************************************

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

******************************************************************************/

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
                reason: format!(
                    "version '{}' should follow semver format (e.g., 1.0.0)",
                    self.version
                ),
            });
        }

        // Validate environment values
        let valid_envs = ["development", "staging", "production"];
        if !valid_envs.contains(&self.environment.as_str()) {
            return Err(ConfigError::ValidationFailed {
                reason: format!("environment must be one of: {}", valid_envs.join(", ")),
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
