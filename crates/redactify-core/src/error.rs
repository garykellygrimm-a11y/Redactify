use thiserror::Error;

/// Errors that redactify-core operations can return.
///
/// `#[non_exhaustive]` means callers must include a wildcard arm when
/// matching — so we can add variants later without breaking them.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RedactifyError {
    /// Manifest could not be serialized to JSON.
    #[error("failed to serialize manifest: {0}")]
    Serialization(#[from] serde_json::Error),
}
