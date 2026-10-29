//! JSON envelope shapes printed to stdout by the headless CLI. The wire
//! format is the agent contract: success is
//! `{"command":…,"ok":true,"result":…}`, failure is
//! `{"command":…,"ok":false,"error":{"kind":…,"message":…,"details"?}}`.
//! There is no human output mode — the TUI is the human interface.

use serde::Serialize;
use serde_json::Value;

use crate::error::{CommandError, ErrorKind};

#[derive(Debug, Serialize)]
pub struct CommandEnvelope<T: Serialize> {
    pub command: &'static str,
    pub ok: bool,
    pub result: T,
}

#[derive(Debug, Serialize)]
pub struct ErrorEnvelope {
    pub command: &'static str,
    pub ok: bool,
    pub error: ErrorBody,
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub kind: ErrorKind,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl ErrorEnvelope {
    pub fn new(command: &'static str, error: &CommandError) -> Self {
        Self {
            command,
            ok: false,
            error: ErrorBody {
                kind: error.kind,
                message: error.message.clone(),
                details: error.details.clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn command_envelope_serializes_success_shape() {
        let value = serde_json::to_value(CommandEnvelope {
            command: "apps.list",
            ok: true,
            result: json!([{ "id": "app-1" }]),
        })
        .unwrap();

        assert_eq!(
            value,
            json!({
                "command": "apps.list",
                "ok": true,
                "result": [{ "id": "app-1" }],
            })
        );
    }

    #[test]
    fn error_envelope_omits_details_when_none() {
        let error = CommandError::usage("bad args");
        let value = serde_json::to_value(ErrorEnvelope::new("auth.login", &error)).unwrap();

        assert_eq!(
            value,
            json!({
                "command": "auth.login",
                "ok": false,
                "error": { "kind": "usage", "message": "bad args" },
            })
        );
    }

    #[test]
    fn error_envelope_includes_details_when_present() {
        let error = CommandError::with_details(
            ErrorKind::Ambiguous,
            "ambiguous app 'grafana': matches 2 entries",
            json!({ "matches": [{ "id": "a" }, { "id": "b" }] }),
        );
        let value = serde_json::to_value(ErrorEnvelope::new("apps.delete", &error)).unwrap();

        assert_eq!(
            value,
            json!({
                "command": "apps.delete",
                "ok": false,
                "error": {
                    "kind": "ambiguous",
                    "message": "ambiguous app 'grafana': matches 2 entries",
                    "details": { "matches": [{ "id": "a" }, { "id": "b" }] },
                },
            })
        );
    }
}
