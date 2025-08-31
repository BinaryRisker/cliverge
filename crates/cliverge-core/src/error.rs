//! Error types for CLIverge core

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Tool installation failed: {0}")]
    InstallationFailed(String),

    #[error("Tool update failed: {0}")]
    UpdateFailed(String),

    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Feature not supported: {0}")]
    NotSupported(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Configuration not found: {0}")]
    NotFound(String),

    #[error("Invalid configuration: {0}")]
    Invalid(String),
}
