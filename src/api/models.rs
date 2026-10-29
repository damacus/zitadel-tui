//! Typed projections of Zitadel API payloads. Every model keeps the original
//! response body in `raw` (attached by the client after decode, never
//! deserialized or serialized as part of the projection).

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip)]
    pub raw: Value,
}

impl RawAttach for Project {
    fn set_raw(&mut self, raw: Value) {
        self.raw = raw;
    }
}

/// Compact `Project` projection — the default envelope shape for
/// `projects list`/`projects get`. All keys are always emitted (null when
/// the field is absent) so agents can rely on a stable wire shape.
#[derive(Clone, Debug, Serialize)]
pub struct ProjectSummary {
    pub id: Option<String>,
    pub name: Option<String>,
    pub state: Option<String>,
}

impl Summarize for Project {
    type Summary = ProjectSummary;
    fn summary(&self) -> ProjectSummary {
        ProjectSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            state: self.state.clone(),
        }
    }
    fn raw(&self) -> &Value {
        &self.raw
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_method_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_types: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_uris: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_method_type: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct App {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oidc_config: Option<OidcConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_config: Option<ApiConfig>,
    #[serde(skip)]
    pub raw: Value,
}

impl RawAttach for App {
    fn set_raw(&mut self, raw: Value) {
        self.raw = raw;
    }
}

impl App {
    /// Client-kind classification preserved from the legacy `classify_oidc_kind`.
    /// OIDC apps are inspected first, then API apps, matching the previous
    /// record mapping order.
    pub fn kind(&self) -> &'static str {
        if let Some(oidc) = &self.oidc_config {
            return oidc.kind();
        }
        if self.api_config.is_some() {
            return "api";
        }
        "unknown"
    }
}

impl OidcConfig {
    fn kind(&self) -> &'static str {
        if self.has_grant_type("OIDC_GRANT_TYPE_DEVICE_CODE") {
            return "device-code";
        }

        if self.app_type.as_deref() == Some("OIDC_APP_TYPE_NATIVE") {
            return "native";
        }

        if let Some(auth_method_type) = self.auth_method_type.as_deref() {
            if auth_method_type == "OIDC_AUTH_METHOD_TYPE_NONE" {
                return "public";
            }
            return "confidential";
        }

        match self.app_type.as_deref() {
            Some("OIDC_APP_TYPE_USER_AGENT") => "public",
            Some("OIDC_APP_TYPE_WEB") => "confidential",
            Some(_) => "oidc",
            None if self.has_useful_fields() => "oidc",
            None => "unknown",
        }
    }

    fn has_grant_type(&self, grant_type: &str) -> bool {
        self.grant_types
            .as_ref()
            .is_some_and(|grant_types| grant_types.iter().any(|value| value == grant_type))
    }

    fn has_useful_fields(&self) -> bool {
        self.client_id.as_deref().is_some_and(|id| !id.is_empty())
            || self
                .redirect_uris
                .as_ref()
                .is_some_and(|uris| !uris.is_empty())
            || self
                .grant_types
                .as_ref()
                .is_some_and(|types| !types.is_empty())
    }
}

/// Compact `App` projection — the default envelope shape for
/// `apps list`/`apps get`. `kind` reuses [`App::kind`]; `client_id` comes
/// from the OIDC config when present, else the API config; `redirect_uris`
/// comes from the OIDC config and defaults to an empty list. All keys are
/// always emitted (null/`[]` when absent) for a stable wire shape.
#[derive(Clone, Debug, Serialize)]
pub struct AppSummary {
    pub id: Option<String>,
    pub name: Option<String>,
    pub client_id: Option<String>,
    pub kind: &'static str,
    pub redirect_uris: Vec<String>,
    pub state: Option<String>,
}

impl Summarize for App {
    type Summary = AppSummary;
    fn summary(&self) -> AppSummary {
        let client_id = self
            .oidc_config
            .as_ref()
            .and_then(|oidc| oidc.client_id.clone())
            .or_else(|| {
                self.api_config
                    .as_ref()
                    .and_then(|api| api.client_id.clone())
            });
        let redirect_uris = self
            .oidc_config
            .as_ref()
            .and_then(|oidc| oidc.redirect_uris.clone())
            .unwrap_or_default();
        AppSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            client_id,
            kind: self.kind(),
            redirect_uris,
            state: self.state.clone(),
        }
    }
    fn raw(&self) -> &Value {
        &self.raw
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HumanProfile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct HumanEmail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HumanUser {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<HumanProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<HumanEmail>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineUser {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_login_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login_names: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub human: Option<HumanUser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine: Option<MachineUser>,
    #[serde(skip)]
    pub raw: Value,
}

impl RawAttach for User {
    fn set_raw(&mut self, raw: Value) {
        self.raw = raw;
    }
}

/// Compact `User` projection — the default envelope shape for
/// `users list`/`users get`. `login_name` comes from `preferred_login_name`
/// with `login_names[0]` as fallback; `display_name` uses the human profile
/// for human users and the machine name for machine users; `email` is only
/// present for human users. All keys are always emitted for a stable wire
/// shape.
#[derive(Clone, Debug, Serialize)]
pub struct UserSummary {
    pub id: Option<String>,
    pub username: Option<String>,
    pub login_name: Option<String>,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub user_type: &'static str,
    pub state: Option<String>,
}

impl User {
    /// `"machine"` when the machine variant is present, else `"human"`.
    pub fn user_type(&self) -> &'static str {
        if self.machine.is_some() {
            return "machine";
        }
        "human"
    }
}

impl Summarize for User {
    type Summary = UserSummary;
    fn summary(&self) -> UserSummary {
        let display_name = self
            .human
            .as_ref()
            .and_then(|human| human.profile.as_ref())
            .and_then(|profile| profile.display_name.clone())
            .or_else(|| {
                self.machine
                    .as_ref()
                    .and_then(|machine| machine.name.clone())
            });
        UserSummary {
            id: self.id.clone(),
            username: self.user_name.clone(),
            login_name: self.preferred_login_name.clone().or_else(|| {
                self.login_names
                    .as_ref()
                    .and_then(|names| names.first().cloned())
            }),
            display_name,
            email: self
                .human
                .as_ref()
                .and_then(|human| human.email.as_ref())
                .and_then(|email| email.email.clone()),
            user_type: self.user_type(),
            state: self.state.clone(),
        }
    }
    fn raw(&self) -> &Value {
        &self.raw
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Idp {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip)]
    pub raw: Value,
}

impl RawAttach for Idp {
    fn set_raw(&mut self, raw: Value) {
        self.raw = raw;
    }
}

/// Compact `Idp` projection — the default envelope shape for
/// `idps list`/`idps get`. All keys are always emitted for a stable wire
/// shape.
#[derive(Clone, Debug, Serialize)]
pub struct IdpSummary {
    pub id: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    pub state: Option<String>,
}

impl Summarize for Idp {
    type Summary = IdpSummary;
    fn summary(&self) -> IdpSummary {
        IdpSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            r#type: self.r#type.clone(),
            state: self.state.clone(),
        }
    }
    fn raw(&self) -> &Value {
        &self.raw
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAppResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    #[serde(skip)]
    pub raw: Value,
}

impl RawAttach for CreateAppResult {
    fn set_raw(&mut self, raw: Value) {
        self.raw = raw;
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip)]
    pub raw: Value,
}

impl RawAttach for CreateUserResult {
    fn set_raw(&mut self, raw: Value) {
        self.raw = raw;
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    #[serde(skip)]
    pub raw: Value,
}

impl RawAttach for SecretResult {
    fn set_raw(&mut self, raw: Value) {
        self.raw = raw;
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_id: Option<String>,
    #[serde(skip)]
    pub raw: Value,
}

impl RawAttach for MemberResult {
    fn set_raw(&mut self, raw: Value) {
        self.raw = raw;
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WhoAmI {
    #[serde(default)]
    pub user: WhoAmIUser,
    #[serde(skip)]
    pub raw: Value,
}

impl RawAttach for WhoAmI {
    fn set_raw(&mut self, raw: Value) {
        self.raw = raw;
    }
}

/// `/auth/v1/users/me` user payload. Zitadel has returned the id as either
/// `userId` or `id`, and the login name as `preferredLoginName`, `userName`,
/// or `loginName`; the aliases cover every observed shape.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WhoAmIUser {
    #[serde(default, alias = "id", skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(
        default,
        alias = "userName",
        alias = "loginName",
        skip_serializing_if = "Option::is_none"
    )]
    pub preferred_login_name: Option<String>,
}

/// Lets the client attach the original response body to a decoded model.
/// `pub(crate)` so only the client can write `raw`; consumers read it for the
/// `--full` projection.
pub(crate) trait RawAttach {
    fn set_raw(&mut self, raw: Value);
}

/// A list/get model with two envelope shapes: a compact typed summary
/// (the default output) and the retained raw API payload (emitted verbatim
/// under `--full`).
pub trait Summarize {
    type Summary: Serialize;
    fn summary(&self) -> Self::Summary;
    fn raw(&self) -> &Value;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn decodes_project() {
        let project: Project = serde_json::from_value(json!({
            "id": "project-1",
            "name": "Platform",
            "state": "PROJECT_STATE_ACTIVE"
        }))
        .expect("project decodes");

        assert_eq!(project.id.as_deref(), Some("project-1"));
        assert_eq!(project.name.as_deref(), Some("Platform"));
        assert_eq!(project.state.as_deref(), Some("PROJECT_STATE_ACTIVE"));
        // raw is attached by the client, not deserialized
        assert_eq!(project.raw, Value::Null);
    }

    #[test]
    fn decodes_oidc_app_with_full_config() {
        let app: App = serde_json::from_value(json!({
            "id": "app-1",
            "name": "grafana",
            "state": "APP_STATE_ACTIVE",
            "oidcConfig": {
                "clientId": "cid-1",
                "authMethodType": "OIDC_AUTH_METHOD_TYPE_BASIC",
                "appType": "OIDC_APP_TYPE_WEB",
                "grantTypes": [
                    "OIDC_GRANT_TYPE_AUTHORIZATION_CODE",
                    "OIDC_GRANT_TYPE_REFRESH_TOKEN"
                ],
                "redirectUris": ["https://grafana.example.com/callback"]
            }
        }))
        .expect("app decodes");

        assert_eq!(app.id.as_deref(), Some("app-1"));
        assert_eq!(app.name.as_deref(), Some("grafana"));
        assert_eq!(app.state.as_deref(), Some("APP_STATE_ACTIVE"));
        let oidc = app.oidc_config.as_ref().expect("oidc config");
        assert_eq!(oidc.client_id.as_deref(), Some("cid-1"));
        assert_eq!(
            oidc.auth_method_type.as_deref(),
            Some("OIDC_AUTH_METHOD_TYPE_BASIC")
        );
        assert_eq!(oidc.app_type.as_deref(), Some("OIDC_APP_TYPE_WEB"));
        assert_eq!(oidc.redirect_uris.as_ref().map(Vec::len), Some(1));
        assert_eq!(app.kind(), "confidential");
    }

    #[test]
    fn decodes_app_with_api_config() {
        let app: App = serde_json::from_value(json!({
            "id": "app-3",
            "name": "Management API",
            "state": "ACTIVE",
            "apiConfig": {
                "clientId": "api-client-1",
                "authMethodType": "API_AUTH_METHOD_TYPE_PRIVATE_KEY_JWT"
            }
        }))
        .expect("api app decodes");

        assert!(app.oidc_config.is_none());
        let api = app.api_config.as_ref().expect("api config");
        assert_eq!(api.client_id.as_deref(), Some("api-client-1"));
        assert_eq!(
            api.auth_method_type.as_deref(),
            Some("API_AUTH_METHOD_TYPE_PRIVATE_KEY_JWT")
        );
        assert_eq!(app.kind(), "api");
    }

    #[test]
    fn decodes_minimal_app_without_configs() {
        let app: App = serde_json::from_value(json!({"id": "app-4", "name": "plain"}))
            .expect("minimal app decodes");

        assert!(app.oidc_config.is_none());
        assert!(app.api_config.is_none());
        assert_eq!(app.state, None);
        assert_eq!(app.kind(), "unknown");
    }

    #[test]
    fn app_kind_matches_legacy_classification() {
        let cases = [
            (
                json!({"grantTypes": ["OIDC_GRANT_TYPE_DEVICE_CODE"], "appType": "OIDC_APP_TYPE_NATIVE"}),
                "device-code",
            ),
            (
                json!({"appType": "OIDC_APP_TYPE_NATIVE", "clientId": "cid"}),
                "native",
            ),
            (
                json!({"authMethodType": "OIDC_AUTH_METHOD_TYPE_NONE"}),
                "public",
            ),
            (
                json!({"authMethodType": "OIDC_AUTH_METHOD_TYPE_BASIC"}),
                "confidential",
            ),
            (json!({"appType": "OIDC_APP_TYPE_USER_AGENT"}), "public"),
            (json!({"appType": "OIDC_APP_TYPE_WEB"}), "confidential"),
            (json!({"appType": "OIDC_APP_TYPE_UNKNOWN_FUTURE"}), "oidc"),
            (json!({"clientId": "cid"}), "oidc"),
            (json!({}), "unknown"),
        ];

        for (oidc_config, expected) in cases {
            let app: App = serde_json::from_value(json!({
                "id": "app",
                "oidcConfig": oidc_config
            }))
            .expect("app decodes");
            assert_eq!(app.kind(), expected, "oidcConfig: {oidc_config}");
        }
    }

    #[test]
    fn decodes_human_user_with_profile_and_email() {
        let user: User = serde_json::from_value(json!({
            "id": "user-1",
            "userName": "admin",
            "state": "USER_STATE_ACTIVE",
            "preferredLoginName": "admin@example.com",
            "human": {
                "profile": {"displayName": "Admin User"},
                "email": {"email": "admin@example.com"}
            }
        }))
        .expect("human user decodes");

        assert_eq!(user.id.as_deref(), Some("user-1"));
        assert_eq!(user.user_name.as_deref(), Some("admin"));
        assert_eq!(
            user.preferred_login_name.as_deref(),
            Some("admin@example.com")
        );
        let human = user.human.as_ref().expect("human details");
        assert_eq!(
            human
                .profile
                .as_ref()
                .and_then(|p| p.display_name.as_deref()),
            Some("Admin User")
        );
        assert_eq!(
            human.email.as_ref().and_then(|e| e.email.as_deref()),
            Some("admin@example.com")
        );
        assert!(user.machine.is_none());
    }

    #[test]
    fn decodes_machine_user_with_login_names() {
        let user: User = serde_json::from_value(json!({
            "id": "user-2",
            "userName": "service-account",
            "state": "ACTIVE",
            "preferredLoginName": "service-account@example.com",
            "loginNames": ["service-account@example.com", "service-account"],
            "machine": {"name": "cluster service account"}
        }))
        .expect("machine user decodes");

        let machine = user.machine.as_ref().expect("machine details");
        assert_eq!(machine.name.as_deref(), Some("cluster service account"));
        assert_eq!(user.login_names.as_ref().map(Vec::len), Some(2));
        assert!(user.human.is_none());
    }

    #[test]
    fn decodes_idp_and_minimal_idp() {
        let idp: Idp = serde_json::from_value(json!({
            "id": "idp-1",
            "name": "Google",
            "state": "IDP_STATE_ACTIVE",
            "type": "IDP_TYPE_GOOGLE"
        }))
        .expect("idp decodes");

        assert_eq!(idp.id.as_deref(), Some("idp-1"));
        assert_eq!(idp.r#type.as_deref(), Some("IDP_TYPE_GOOGLE"));

        let minimal: Idp = serde_json::from_value(json!({"id": "idp-2"})).expect("minimal idp");
        assert_eq!(minimal.id.as_deref(), Some("idp-2"));
        assert_eq!(minimal.r#type, None);
        assert_eq!(minimal.state, None);
    }

    #[test]
    fn decodes_create_app_result_with_optional_secret() {
        let result: CreateAppResult = serde_json::from_value(json!({
            "appId": "app-1",
            "clientId": "cid-1",
            "clientSecret": "secret-1"
        }))
        .expect("create app result decodes");

        assert_eq!(result.app_id.as_deref(), Some("app-1"));
        assert_eq!(result.client_id.as_deref(), Some("cid-1"));
        assert_eq!(result.client_secret.as_deref(), Some("secret-1"));

        let no_secret: CreateAppResult = serde_json::from_value(json!({"clientId": "cid-2"}))
            .expect("missing fields still decode");
        assert_eq!(no_secret.app_id, None);
        assert_eq!(no_secret.client_secret, None);
    }

    #[test]
    fn decodes_create_user_member_and_secret_results() {
        let user: CreateUserResult =
            serde_json::from_value(json!({"userId": "user-1"})).expect("user result");
        assert_eq!(user.user_id.as_deref(), Some("user-1"));

        let member: MemberResult =
            serde_json::from_value(json!({"memberId": "member-1"})).expect("member result");
        assert_eq!(member.member_id.as_deref(), Some("member-1"));
        let empty_member: MemberResult =
            serde_json::from_value(json!({})).expect("empty member body decodes");
        assert_eq!(empty_member.member_id, None);

        let secret: SecretResult =
            serde_json::from_value(json!({"clientSecret": "new-secret"})).expect("secret result");
        assert_eq!(secret.client_secret.as_deref(), Some("new-secret"));
    }

    #[test]
    fn decodes_whoami_with_user_id_aliases() {
        let canonical: WhoAmI = serde_json::from_value(json!({
            "user": {"userId": "u-1", "preferredLoginName": "admin@example.com"}
        }))
        .expect("canonical whoami");
        assert_eq!(canonical.user.user_id.as_deref(), Some("u-1"));
        assert_eq!(
            canonical.user.preferred_login_name.as_deref(),
            Some("admin@example.com")
        );

        let aliased: WhoAmI = serde_json::from_value(json!({
            "user": {"id": "u-2", "userName": "sa-user"}
        }))
        .expect("aliased whoami");
        assert_eq!(aliased.user.user_id.as_deref(), Some("u-2"));
        assert_eq!(
            aliased.user.preferred_login_name.as_deref(),
            Some("sa-user")
        );

        let login_name: WhoAmI = serde_json::from_value(json!({
            "user": {"id": "u-3", "loginName": "sa-login"}
        }))
        .expect("loginName alias");
        assert_eq!(
            login_name.user.preferred_login_name.as_deref(),
            Some("sa-login")
        );

        let missing_user: WhoAmI = serde_json::from_value(json!({})).expect("empty whoami");
        assert_eq!(missing_user.user.user_id, None);
    }

    #[test]
    fn serialized_models_preserve_cli_output_keys() {
        let app_fixture = json!({
            "id": "app-1",
            "name": "grafana",
            "state": "ACTIVE",
            "oidcConfig": {
                "clientId": "cid-1",
                "authMethodType": "OIDC_AUTH_METHOD_TYPE_NONE",
                "appType": "OIDC_APP_TYPE_WEB",
                "grantTypes": ["OIDC_GRANT_TYPE_AUTHORIZATION_CODE"],
                "redirectUris": ["https://grafana.example.com/callback"]
            }
        });
        let app: App = serde_json::from_value(app_fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&app).unwrap(), app_fixture);

        let user_fixture = json!({
            "id": "user-1",
            "userName": "service-account",
            "state": "ACTIVE",
            "preferredLoginName": "sa@example.com",
            "loginNames": ["sa@example.com"],
            "machine": {"name": "sa"}
        });
        let user: User = serde_json::from_value(user_fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&user).unwrap(), user_fixture);

        let idp_fixture = json!({
            "id": "idp-1",
            "name": "Google",
            "state": "ACTIVE",
            "type": "google"
        });
        let idp: Idp = serde_json::from_value(idp_fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(&idp).unwrap(), idp_fixture);
    }

    #[test]
    fn raw_payload_is_attached_by_client_not_deserializer() {
        let mut app: App = serde_json::from_value(json!({"id": "app-1"})).unwrap();
        assert_eq!(app.raw, serde_json::Value::Null);
        let payload = json!({"id": "app-1", "extra": "kept"});
        app.set_raw(payload.clone());
        assert_eq!(app.raw, payload);
        // raw is never serialized into the typed projection
        assert_eq!(serde_json::to_value(&app).unwrap(), json!({"id": "app-1"}));
    }

    #[test]
    fn app_summary_projects_compact_shape() {
        let app: App = serde_json::from_value(json!({
            "id": "app-1",
            "name": "grafana",
            "state": "APP_STATE_ACTIVE",
            "oidcConfig": {
                "clientId": "cid-1",
                "authMethodType": "OIDC_AUTH_METHOD_TYPE_BASIC",
                "appType": "OIDC_APP_TYPE_WEB",
                "redirectUris": ["https://grafana.example.com/callback"]
            },
            "creationDate": "2026-01-01T00:00:00Z"
        }))
        .expect("app decodes");

        assert_eq!(
            serde_json::to_value(app.summary()).unwrap(),
            json!({
                "id": "app-1",
                "name": "grafana",
                "client_id": "cid-1",
                "kind": "confidential",
                "redirect_uris": ["https://grafana.example.com/callback"],
                "state": "APP_STATE_ACTIVE"
            })
        );
    }

    #[test]
    fn app_summary_uses_api_config_client_id_and_empty_redirects() {
        let app: App = serde_json::from_value(json!({
            "id": "app-2",
            "name": "Management API",
            "apiConfig": {
                "clientId": "api-client-1",
                "authMethodType": "API_AUTH_METHOD_TYPE_PRIVATE_KEY_JWT"
            }
        }))
        .expect("api app decodes");

        assert_eq!(
            serde_json::to_value(app.summary()).unwrap(),
            json!({
                "id": "app-2",
                "name": "Management API",
                "client_id": "api-client-1",
                "kind": "api",
                "redirect_uris": [],
                "state": null
            })
        );
    }

    #[test]
    fn app_summary_minimal_app_has_stable_keys() {
        let app: App = serde_json::from_value(json!({"id": "app-3"})).expect("minimal app");

        assert_eq!(
            serde_json::to_value(app.summary()).unwrap(),
            json!({
                "id": "app-3",
                "name": null,
                "client_id": null,
                "kind": "unknown",
                "redirect_uris": [],
                "state": null
            })
        );
    }

    #[test]
    fn user_summary_projects_human_user() {
        let user: User = serde_json::from_value(json!({
            "id": "user-1",
            "userName": "admin",
            "state": "USER_STATE_ACTIVE",
            "preferredLoginName": "admin@example.com",
            "human": {
                "profile": {"displayName": "Admin User"},
                "email": {"email": "admin@example.com"}
            }
        }))
        .expect("human user decodes");

        assert_eq!(
            serde_json::to_value(user.summary()).unwrap(),
            json!({
                "id": "user-1",
                "username": "admin",
                "login_name": "admin@example.com",
                "display_name": "Admin User",
                "email": "admin@example.com",
                "user_type": "human",
                "state": "USER_STATE_ACTIVE"
            })
        );
    }

    #[test]
    fn user_summary_projects_machine_user() {
        let user: User = serde_json::from_value(json!({
            "id": "user-2",
            "userName": "service-account",
            "state": "USER_STATE_ACTIVE",
            "loginNames": ["sa@example.com"],
            "machine": {"name": "cluster service account"}
        }))
        .expect("machine user decodes");

        assert_eq!(
            serde_json::to_value(user.summary()).unwrap(),
            json!({
                "id": "user-2",
                "username": "service-account",
                "login_name": "sa@example.com",
                "display_name": "cluster service account",
                "email": null,
                "user_type": "machine",
                "state": "USER_STATE_ACTIVE"
            })
        );
    }

    #[test]
    fn idp_summary_projects_compact_shape() {
        let idp: Idp = serde_json::from_value(json!({
            "id": "idp-1",
            "name": "Google",
            "state": "IDP_STATE_ACTIVE",
            "type": "IDP_TYPE_GOOGLE"
        }))
        .expect("idp decodes");

        assert_eq!(
            serde_json::to_value(idp.summary()).unwrap(),
            json!({
                "id": "idp-1",
                "name": "Google",
                "type": "IDP_TYPE_GOOGLE",
                "state": "IDP_STATE_ACTIVE"
            })
        );
    }

    #[test]
    fn project_summary_projects_compact_shape() {
        let project: Project = serde_json::from_value(json!({
            "id": "project-1",
            "name": "Platform",
            "state": "PROJECT_STATE_ACTIVE",
            "creationDate": "2026-01-01T00:00:00Z"
        }))
        .expect("project decodes");

        assert_eq!(
            serde_json::to_value(project.summary()).unwrap(),
            json!({
                "id": "project-1",
                "name": "Platform",
                "state": "PROJECT_STATE_ACTIVE"
            })
        );
    }
}
