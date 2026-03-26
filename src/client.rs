use anyhow::{bail, Context, Result};
use reqwest::{Client, Method};
use serde_json::{json, Value};

pub struct ZitadelClient {
    http: Client,
    base_url: String,
    token: String,
}

impl ZitadelClient {
    pub fn new(base_url: String, token: String) -> Result<Self> {
        Ok(Self {
            http: Client::builder().build()?,
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
        })
    }

    pub async fn whoami(&self) -> Result<Value> {
        self.api_request(Method::GET, "/auth/v1/users/me", None)
            .await
    }

    pub async fn list_projects(&self) -> Result<Vec<Value>> {
        let response = self
            .api_request(
                Method::POST,
                "/management/v1/projects/_search",
                Some(json!({ "query": { "limit": 100 } })),
            )
            .await?;
        Ok(result_array(response))
    }

    pub async fn get_default_project(&self) -> Result<Value> {
        let projects = self.list_projects().await?;
        projects
            .into_iter()
            .next()
            .context("no projects found in Zitadel")
    }

    pub async fn list_apps(&self, project_id: &str) -> Result<Vec<Value>> {
        let response = self
            .api_request(
                Method::POST,
                &format!("/management/v1/projects/{project_id}/apps/_search"),
                Some(json!({ "query": { "limit": 100 } })),
            )
            .await?;
        Ok(result_array(response))
    }

    pub async fn create_oidc_app(
        &self,
        project_id: &str,
        name: &str,
        redirect_uris: Vec<String>,
        public: bool,
    ) -> Result<Value> {
        let auth_method = if public {
            "OIDC_AUTH_METHOD_TYPE_NONE"
        } else {
            "OIDC_AUTH_METHOD_TYPE_BASIC"
        };
        let app_type = if public {
            "OIDC_APP_TYPE_USER_AGENT"
        } else {
            "OIDC_APP_TYPE_WEB"
        };

        self.api_request(
            Method::POST,
            &format!("/management/v1/projects/{project_id}/apps/oidc"),
            Some(json!({
                "name": name,
                "redirectUris": redirect_uris,
                "responseTypes": ["OIDC_RESPONSE_TYPE_CODE"],
                "grantTypes": ["OIDC_GRANT_TYPE_AUTHORIZATION_CODE", "OIDC_GRANT_TYPE_REFRESH_TOKEN"],
                "appType": app_type,
                "authMethodType": auth_method,
                "postLogoutRedirectUris": [],
                "version": "OIDC_VERSION_1_0",
                "devMode": false,
                "accessTokenType": "OIDC_TOKEN_TYPE_BEARER",
                "accessTokenRoleAssertion": true,
                "idTokenRoleAssertion": true,
                "idTokenUserinfoAssertion": true,
                "clockSkew": "1s",
                "additionalOrigins": []
            })),
        )
        .await
    }

    pub async fn create_native_app(
        &self,
        project_id: &str,
        name: &str,
        device_code: bool,
    ) -> Result<Value> {
        let mut grant_types = vec!["OIDC_GRANT_TYPE_REFRESH_TOKEN"];
        if device_code {
            grant_types.push("OIDC_GRANT_TYPE_DEVICE_CODE");
        }
        let access_token_type = if device_code {
            "OIDC_TOKEN_TYPE_JWT"
        } else {
            "OIDC_TOKEN_TYPE_BEARER"
        };
        self.api_request(
            Method::POST,
            &format!("/management/v1/projects/{project_id}/apps/oidc"),
            Some(json!({
                "name": name,
                "redirectUris": [],
                "responseTypes": ["OIDC_RESPONSE_TYPE_CODE"],
                "grantTypes": grant_types,
                "appType": "OIDC_APP_TYPE_NATIVE",
                "authMethodType": "OIDC_AUTH_METHOD_TYPE_NONE",
                "postLogoutRedirectUris": [],
                "version": "OIDC_VERSION_1_0",
                "devMode": false,
                "accessTokenType": access_token_type,
                "accessTokenRoleAssertion": true,
                "idTokenRoleAssertion": true,
                "idTokenUserinfoAssertion": true,
                "clockSkew": "1s",
                "additionalOrigins": []
            })),
        )
        .await
    }

    pub async fn delete_app(&self, project_id: &str, app_id: &str) -> Result<Value> {
        self.api_request(
            Method::DELETE,
            &format!("/management/v1/projects/{project_id}/apps/{app_id}"),
            None,
        )
        .await
    }

    pub async fn regenerate_secret(&self, project_id: &str, app_id: &str) -> Result<Value> {
        self.api_request(
            Method::PUT,
            &format!("/management/v1/projects/{project_id}/apps/{app_id}/oidc_config/secret"),
            None,
        )
        .await
    }

    pub async fn list_users(&self, limit: u64) -> Result<Vec<Value>> {
        let response = self
            .api_request(
                Method::POST,
                "/management/v1/users/_search",
                Some(json!({ "query": { "limit": limit } })),
            )
            .await?;
        Ok(result_array(response))
    }

    pub async fn create_human_user(
        &self,
        email: &str,
        first_name: &str,
        last_name: &str,
        username: Option<&str>,
    ) -> Result<Value> {
        self.api_request(
            Method::POST,
            "/v2/users/human",
            Some(json!({
                "username": username.unwrap_or_else(|| email.split('@').next().unwrap_or(email)),
                "profile": {
                    "givenName": first_name,
                    "familyName": last_name
                },
                "email": {
                    "email": email,
                    "isVerified": true
                }
            })),
        )
        .await
    }

    pub async fn import_human_user(
        &self,
        username: &str,
        first_name: &str,
        last_name: &str,
        email: &str,
        password: &str,
        password_change_required: bool,
    ) -> Result<Value> {
        self.api_request(
            Method::POST,
            "/management/v1/users/human/_import",
            Some(json!({
                "userName": username,
                "profile": {
                    "firstName": first_name,
                    "lastName": last_name,
                    "displayName": format!("{first_name} {last_name}")
                },
                "email": {
                    "email": email,
                    "isEmailVerified": true
                },
                "password": password,
                "passwordChangeRequired": password_change_required
            })),
        )
        .await
    }

    pub async fn grant_iam_owner(&self, user_id: &str) -> Result<Value> {
        self.api_request(
            Method::POST,
            "/admin/v1/members",
            Some(json!({ "userId": user_id, "roles": ["IAM_OWNER"] })),
        )
        .await
    }

    pub async fn list_idps(&self) -> Result<Vec<Value>> {
        let response = self
            .api_request(
                Method::POST,
                "/admin/v1/idps/_search",
                Some(json!({ "query": { "limit": 100 } })),
            )
            .await?;
        Ok(result_array(response))
    }

    pub async fn add_google_idp(
        &self,
        client_id: &str,
        client_secret: &str,
        name: &str,
    ) -> Result<Value> {
        self.api_request(
            Method::POST,
            "/admin/v1/idps/google",
            Some(json!({
                "name": name,
                "clientId": client_id,
                "clientSecret": client_secret,
                "scopes": ["openid", "profile", "email"],
                "providerOptions": {
                    "isLinkingAllowed": true,
                    "isCreationAllowed": true,
                    "isAutoCreation": false,
                    "isAutoUpdate": true
                }
            })),
        )
        .await
    }

    async fn api_request(&self, method: Method, path: &str, body: Option<Value>) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self
            .http
            .request(method, url)
            .bearer_auth(&self.token)
            .header("Accept", "application/json");

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await?;
        let status = response.status();
        let bytes = response.bytes().await?;
        if !status.is_success() {
            if status == reqwest::StatusCode::UNAUTHORIZED {
                bail!(
                    "Authentication failed (401 Unauthorized). \
                     Check your credentials or run `auth login` to authenticate."
                );
            }
            bail!("API request failed ({status})");
        }

        if bytes.is_empty() {
            return Ok(json!({}));
        }

        serde_json::from_slice(&bytes).context("failed to decode Zitadel response")
    }
}

fn result_array(value: Value) -> Vec<Value> {
    value
        .get("result")
        .and_then(|result| result.as_array())
        .cloned()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server};

    #[tokio::test]
    async fn list_projects_sends_bearer_token_and_parses_results() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/management/v1/projects/_search")
            .match_header("authorization", "Bearer test-token")
            .match_body(Matcher::Json(json!({ "query": { "limit": 100 } })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"result":[{"id":"p1","name":"Platform"}]}"#)
            .create_async()
            .await;

        let client = ZitadelClient::new(server.url(), "test-token".to_string()).unwrap();
        let projects = client.list_projects().await.unwrap();

        mock.assert_async().await;
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0]["id"], "p1");
    }

    #[tokio::test]
    async fn create_oidc_app_uses_expected_endpoint() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/management/v1/projects/project-1/apps/oidc")
            .match_header("authorization", "Bearer test-token")
            .match_body(Matcher::PartialJson(json!({
                "name": "grafana",
                "redirectUris": ["https://grafana.example.com/callback"],
                "authMethodType": "OIDC_AUTH_METHOD_TYPE_BASIC"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"appId":"app-1","clientId":"cid-1"}"#)
            .create_async()
            .await;

        let client = ZitadelClient::new(server.url(), "test-token".to_string()).unwrap();
        let created = client
            .create_oidc_app(
                "project-1",
                "grafana",
                vec!["https://grafana.example.com/callback".to_string()],
                false,
            )
            .await
            .unwrap();

        mock.assert_async().await;
        assert_eq!(created["appId"], "app-1");
        assert_eq!(created["clientId"], "cid-1");
    }

    #[tokio::test]
    async fn surfaces_api_errors_with_status_and_body() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/admin/v1/idps/_search")
            .with_status(403)
            .with_header("content-type", "application/json")
            .with_body(r#"{"message":"forbidden","client_secret":"leaked"}"#)
            .create_async()
            .await;

        let client = ZitadelClient::new(server.url(), "test-token".to_string()).unwrap();
        let error = client.list_idps().await.unwrap_err().to_string();

        mock.assert_async().await;
        assert!(error.contains("403"));
        assert!(!error.contains("forbidden"));
        assert!(!error.contains("leaked"));
    }

    #[tokio::test]
    async fn create_native_app_uses_jwt_tokens_for_device_code_clients() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/management/v1/projects/project-1/apps/oidc")
            .match_header("authorization", "Bearer test-token")
            .match_body(Matcher::PartialJson(json!({
                "name": "zitadel-tui",
                "grantTypes": [
                    "OIDC_GRANT_TYPE_REFRESH_TOKEN",
                    "OIDC_GRANT_TYPE_DEVICE_CODE"
                ],
                "accessTokenType": "OIDC_TOKEN_TYPE_JWT"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"appId":"app-1","clientId":"cid-1"}"#)
            .create_async()
            .await;

        let client = ZitadelClient::new(server.url(), "test-token".to_string()).unwrap();
        let created = client
            .create_native_app("project-1", "zitadel-tui", true)
            .await
            .unwrap();

        mock.assert_async().await;
        assert_eq!(created["clientId"], "cid-1");
    }

    #[tokio::test]
    async fn create_native_app_keeps_bearer_tokens_without_device_code() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/management/v1/projects/project-1/apps/oidc")
            .match_header("authorization", "Bearer test-token")
            .match_body(Matcher::PartialJson(json!({
                "name": "desktop-app",
                "grantTypes": ["OIDC_GRANT_TYPE_REFRESH_TOKEN"],
                "accessTokenType": "OIDC_TOKEN_TYPE_BEARER"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"appId":"app-2","clientId":"cid-2"}"#)
            .create_async()
            .await;

        let client = ZitadelClient::new(server.url(), "test-token".to_string()).unwrap();
        let created = client
            .create_native_app("project-1", "desktop-app", false)
            .await
            .unwrap();

        mock.assert_async().await;
        assert_eq!(created["clientId"], "cid-2");
    }

    #[tokio::test]
    async fn empty_2xx_body_is_treated_as_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("DELETE", "/management/v1/projects/project-1/apps/app-1")
            .match_header("authorization", "Bearer test-token")
            .with_status(204)
            .create_async()
            .await;

        let client = ZitadelClient::new(server.url(), "test-token".to_string()).unwrap();
        let response = client.delete_app("project-1", "app-1").await.unwrap();

        mock.assert_async().await;
        assert_eq!(response, json!({}));
    }

    #[tokio::test]
    async fn malformed_json_is_reported_without_echoing_input() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/management/v1/users/_search")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"result": [}"#)
            .create_async()
            .await;

        let client = ZitadelClient::new(server.url(), "test-token".to_string()).unwrap();
        let error = client.list_users(100).await.unwrap_err().to_string();

        mock.assert_async().await;
        assert!(error.contains("failed to decode Zitadel response"));
        assert!(!error.contains("result"));
    }

    #[tokio::test]
    async fn grant_iam_owner_sends_expected_payload() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/admin/v1/members")
            .match_header("authorization", "Bearer test-token")
            .match_body(Matcher::PartialJson(json!({
                "userId": "user-1",
                "roles": ["IAM_OWNER"]
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"memberId":"member-1"}"#)
            .create_async()
            .await;

        let client = ZitadelClient::new(server.url(), "test-token".to_string()).unwrap();
        let response = client.grant_iam_owner("user-1").await.unwrap();

        mock.assert_async().await;
        assert_eq!(response["memberId"], "member-1");
    }
}
