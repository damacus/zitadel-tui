//! Typed error model for the headless command boundary. `ErrorKind` wire
//! values (snake_case) and the `error.details` shapes are part of the agent
//! contract — keep them stable.

use serde::Serialize;
use serde_json::Value;

/// Classification surfaced as `error.kind` in the JSON envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// Missing/invalid inputs clap cannot express (e.g. `auth login`
    /// without a client id, `apps create` without `--redirect-uris`).
    Usage,
    /// Credential resolution failures (no usable PAT/service account/
    /// session token, OIDC session used for API commands).
    Auth,
    /// A named resource resolved to zero matches.
    NotFound,
    /// A selector matched more than one resource.
    Ambiguous,
    /// Zitadel API / transport / decode failures (the fallback kind).
    Api,
    /// Config file load/save/parse failures.
    Config,
    /// Local filesystem failures outside config (e.g. token cache).
    Io,
}

/// Error produced at the `commands/` boundary; `main.rs` turns it into the
/// error envelope.
#[derive(Debug)]
pub struct CommandError {
    pub kind: ErrorKind,
    pub message: String,
    pub details: Option<Value>,
}

impl CommandError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(kind: ErrorKind, message: impl Into<String>, details: Value) -> Self {
        Self {
            kind,
            message: message.into(),
            details: Some(details),
        }
    }

    pub fn usage(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Usage, message)
    }

    pub fn auth(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Auth, message)
    }

    pub fn config(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Config, message)
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Io, message)
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CommandError {}

/// Any anyhow error reaching the boundary without a more specific mapping
/// is an API failure.
impl From<anyhow::Error> for CommandError {
    fn from(error: anyhow::Error) -> Self {
        Self::new(ErrorKind::Api, error.to_string())
    }
}

/// Serialization failures while building the envelope `result` are
/// internal/API-class failures.
impl From<serde_json::Error> for CommandError {
    fn from(error: serde_json::Error) -> Self {
        Self::new(ErrorKind::Api, error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn error_kind_serializes_snake_case() {
        let cases = [
            (ErrorKind::Usage, "usage"),
            (ErrorKind::Auth, "auth"),
            (ErrorKind::NotFound, "not_found"),
            (ErrorKind::Ambiguous, "ambiguous"),
            (ErrorKind::Api, "api"),
            (ErrorKind::Config, "config"),
            (ErrorKind::Io, "io"),
        ];
        for (kind, expected) in cases {
            assert_eq!(serde_json::to_value(kind).unwrap(), json!(expected));
        }
    }

    #[test]
    fn anyhow_errors_fall_back_to_api_kind() {
        let error = CommandError::from(anyhow::anyhow!("API request failed (500)"));
        assert_eq!(error.kind, ErrorKind::Api);
        assert_eq!(error.message, "API request failed (500)");
        assert!(error.details.is_none());
    }

    #[test]
    fn display_renders_the_message() {
        let error = CommandError::usage("bad args");
        assert_eq!(error.to_string(), "bad args");
    }
}
