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

    /// Rules file could not be read from disk.
    #[error("could not read rules file '{path}': {source}")]
    RulesIo {
        path: String,
        source: std::io::Error,
    },

    /// Rules file is not valid TOML / does not match the expected schema.
    /// The toml error carries line and column information.
    #[error("invalid rules file: {0}")]
    RulesParse(#[from] toml::de::Error),

    /// A user-supplied pattern failed to compile (bad syntax, unsupported
    /// feature such as lookaround, or exceeded the compile-size limit).
    #[error("invalid pattern in rule '{id}': {source}")]
    InvalidRule { id: String, source: regex::Error },

    /// Two rules in the same file share an id.
    #[error("duplicate rule id '{id}' in rules file")]
    DuplicateRuleId { id: String },
}
