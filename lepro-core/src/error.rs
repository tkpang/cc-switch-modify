//! Error types for lepro-core.

use thiserror::Error;

/// The error type for lepro-core operations.
#[derive(Debug, Error)]
pub enum LeproError {
    /// HTTP-related errors.
    #[error("http: {0}")]
    Http(String),

    /// Parsing errors.
    #[error("parse: {0}")]
    Parse(String),

    /// OIDC-related errors.
    #[error("oidc: {0}")]
    Oidc(String),

    /// I/O errors.
    #[error("io: {0}")]
    Io(String),
}
