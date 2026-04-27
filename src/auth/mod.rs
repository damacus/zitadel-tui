mod resolver;
mod service_account;
mod session;

#[derive(Debug, Clone)]
pub struct ResolvedAuth {
    pub token: String,
    pub source: &'static str,
}

pub use resolver::resolve_access_token;
pub(crate) use session::is_authentication_required_response;
pub use session::validate_login_session_token;
