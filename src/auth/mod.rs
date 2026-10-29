mod resolver;
mod service_account;
mod session;

use anyhow::{bail, Result};

const API_CREDENTIAL_REQUIRED: &str = "This command requires Zitadel API credentials. OIDC device-login sessions can only run `auth status`, `auth login`, and `auth logout`; use a personal access token or service account token for Apps, Users, IDP, Auth API, Management API, or Admin API operations.";

/// Where the resolved credential came from. The `Display` strings are part of
/// the CLI JSON output (`auth status`) and the TUI auth label — keep them
/// stable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthSource {
    CliPat,
    EnvPat,
    ConfigPat,
    CliServiceAccount,
    EnvServiceAccount,
    ConfigServiceAccount,
    SessionToken,
    RefreshedSessionToken,
}

impl AuthSource {
    /// OIDC device-login session tokens authenticate a user but cannot call the
    /// Zitadel Auth, Management, or Admin APIs.
    pub fn is_oidc_session(&self) -> bool {
        matches!(self, Self::SessionToken | Self::RefreshedSessionToken)
    }
}

impl std::fmt::Display for AuthSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::CliPat => "cli PAT",
            Self::EnvPat => "env PAT",
            Self::ConfigPat => "config PAT",
            Self::CliServiceAccount => "cli service account",
            Self::EnvServiceAccount => "env service account",
            Self::ConfigServiceAccount => "config service account",
            Self::SessionToken => "session token",
            Self::RefreshedSessionToken => "session token (refreshed)",
        })
    }
}

impl serde::Serialize for AuthSource {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedAuth {
    pub token: String,
    pub source: AuthSource,
}

impl ResolvedAuth {
    pub fn is_oidc_session(&self) -> bool {
        self.source.is_oidc_session()
    }

    pub fn ensure_api_credential(&self) -> Result<()> {
        if self.is_oidc_session() {
            bail!(API_CREDENTIAL_REQUIRED);
        }

        Ok(())
    }
}

pub use resolver::resolve_access_token;
pub(crate) use session::is_authentication_required_response;
pub use session::validate_login_session_token;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_source_display_labels_are_stable() {
        let cases = [
            (AuthSource::CliPat, "cli PAT"),
            (AuthSource::EnvPat, "env PAT"),
            (AuthSource::ConfigPat, "config PAT"),
            (AuthSource::CliServiceAccount, "cli service account"),
            (AuthSource::EnvServiceAccount, "env service account"),
            (AuthSource::ConfigServiceAccount, "config service account"),
            (AuthSource::SessionToken, "session token"),
            (
                AuthSource::RefreshedSessionToken,
                "session token (refreshed)",
            ),
        ];
        for (source, expected) in cases {
            assert_eq!(source.to_string(), expected);
            assert_eq!(serde_json::to_value(source).unwrap(), expected);
        }
    }

    #[test]
    fn only_session_sources_are_oidc_sessions() {
        let api_sources = [
            AuthSource::CliPat,
            AuthSource::EnvPat,
            AuthSource::ConfigPat,
            AuthSource::CliServiceAccount,
            AuthSource::EnvServiceAccount,
            AuthSource::ConfigServiceAccount,
        ];
        for source in api_sources {
            assert!(!source.is_oidc_session(), "{source}");
        }
        assert!(AuthSource::SessionToken.is_oidc_session());
        assert!(AuthSource::RefreshedSessionToken.is_oidc_session());
    }

    #[test]
    fn resolved_auth_delegates_oidc_session_check_to_source() {
        let session = ResolvedAuth {
            token: "t".to_string(),
            source: AuthSource::SessionToken,
        };
        assert!(session.is_oidc_session());

        let pat = ResolvedAuth {
            token: "t".to_string(),
            source: AuthSource::CliPat,
        };
        assert!(!pat.is_oidc_session());
    }
}
