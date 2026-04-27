mod resolver;
mod service_account;
mod session;

use anyhow::{bail, Result};

const API_CREDENTIAL_REQUIRED: &str = "This command requires Zitadel API credentials. OIDC device-login sessions can only run `auth status`, `auth login`, and `auth logout`; use a personal access token or service account token for Apps, Users, IDP, Auth API, Management API, or Admin API operations.";

#[derive(Debug, Clone)]
pub struct ResolvedAuth {
    pub token: String,
    pub source: &'static str,
}

impl ResolvedAuth {
    pub fn is_oidc_session(&self) -> bool {
        self.source.starts_with("session token")
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
