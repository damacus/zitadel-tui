use std::collections::BTreeMap;

use anyhow::{Context, Result};

use crate::{
    api::{
        client::ZitadelClient,
        models::{App, CreateAppResult, SecretResult},
    },
    config::AppTemplate,
};

use super::{resolve_app, AppSelector};

/// Lists apps in a project, up to `limit`.
pub async fn list(client: &ZitadelClient, project_id: &str, limit: u64) -> Result<Vec<App>> {
    client.list_apps(project_id, limit).await
}

/// Looks up a single app by selector, erroring on zero or multiple matches.
pub async fn get(client: &ZitadelClient, project_id: &str, selector: &AppSelector) -> Result<App> {
    resolve_app(client, project_id, selector)
        .await?
        .into_result(&selector.describe())
}

/// Creates a web/user-agent OIDC application.
pub async fn create_oidc(
    client: &ZitadelClient,
    project_id: &str,
    name: &str,
    redirect_uris: Vec<String>,
    public: bool,
) -> Result<CreateAppResult> {
    client
        .create_oidc_app(project_id, name, redirect_uris, public)
        .await
}

/// Creates a native OIDC application, optionally with the device-code grant.
pub async fn create_native(
    client: &ZitadelClient,
    project_id: &str,
    name: &str,
    device_code: bool,
) -> Result<CreateAppResult> {
    client
        .create_native_app(project_id, name, device_code)
        .await
}

/// Deletes an app by id.
pub async fn delete(client: &ZitadelClient, project_id: &str, app_id: &str) -> Result<()> {
    client.delete_app(project_id, app_id).await
}

/// Regenerates the confidential client secret for an app.
pub async fn regenerate_secret(
    client: &ZitadelClient,
    project_id: &str,
    app_id: &str,
) -> Result<SecretResult> {
    client.regenerate_secret(project_id, app_id).await
}

/// Creates one app per requested template name, sequentially, in the given
/// order; results align with `names` by index.
pub async fn quick_setup(
    client: &ZitadelClient,
    project_id: &str,
    templates: &BTreeMap<String, AppTemplate>,
    names: &[String],
) -> Result<Vec<CreateAppResult>> {
    let mut created = Vec::with_capacity(names.len());
    for name in names {
        let template = templates
            .get(name)
            .with_context(|| format!("template {name} not found"))?;
        let result = client
            .create_oidc_app(
                project_id,
                name,
                template.redirect_uris.clone(),
                template.public,
            )
            .await
            .map_err(|error| anyhow::anyhow!("{name} failed: {error}"))?;
        created.push(result);
    }
    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server};
    use serde_json::json;

    fn client(server: &Server) -> ZitadelClient {
        ZitadelClient::new(server.url(), "test-token".to_string()).unwrap()
    }

    fn template(redirect_uris: &[&str], public: bool) -> AppTemplate {
        AppTemplate {
            redirect_uris: redirect_uris.iter().map(|uri| uri.to_string()).collect(),
            public,
        }
    }

    #[tokio::test]
    async fn get_resolves_app_by_name() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/management/v1/projects/project-1/apps/_search")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"result":[{"id":"app-1","name":"grafana"}]}"#)
            .create_async()
            .await;

        let app = get(
            &client(&server),
            "project-1",
            &AppSelector::Name("grafana".to_string()),
        )
        .await
        .unwrap();

        assert_eq!(app.id.as_deref(), Some("app-1"));
    }

    #[tokio::test]
    async fn get_errors_with_not_found_text() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/management/v1/projects/project-1/apps/_search")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"result":[]}"#)
            .create_async()
            .await;

        let error = get(
            &client(&server),
            "project-1",
            &AppSelector::Name("ghost".to_string()),
        )
        .await
        .unwrap_err()
        .to_string();

        assert_eq!(error, "app 'ghost' not found");
    }

    #[tokio::test]
    async fn delete_calls_delete_endpoint() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("DELETE", "/management/v1/projects/project-1/apps/app-1")
            .match_header("authorization", "Bearer test-token")
            .with_status(204)
            .create_async()
            .await;

        delete(&client(&server), "project-1", "app-1")
            .await
            .unwrap();

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn regenerate_secret_returns_secret_result() {
        let mut server = Server::new_async().await;
        server
            .mock(
                "POST",
                "/management/v1/projects/project-1/apps/app-1/oidc/secret/_regenerate",
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"clientSecret":"new-secret"}"#)
            .create_async()
            .await;

        let secret = regenerate_secret(&client(&server), "project-1", "app-1")
            .await
            .unwrap();

        assert_eq!(secret.client_secret.as_deref(), Some("new-secret"));
    }

    #[tokio::test]
    async fn quick_setup_creates_templates_in_requested_order() {
        let mut server = Server::new_async().await;
        let grafana = server
            .mock("POST", "/management/v1/projects/project-1/apps/oidc")
            .match_body(Matcher::PartialJson(json!({
                "name": "grafana",
                "redirectUris": ["https://grafana.example.com/callback"],
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"appId":"app-1","clientId":"cid-grafana"}"#)
            .create_async()
            .await;
        let mealie = server
            .mock("POST", "/management/v1/projects/project-1/apps/oidc")
            .match_body(Matcher::PartialJson(json!({
                "name": "mealie",
                "redirectUris": ["https://mealie.example.com/callback"],
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"appId":"app-2","clientId":"cid-mealie"}"#)
            .create_async()
            .await;

        let mut templates = BTreeMap::new();
        templates.insert(
            "grafana".to_string(),
            template(&["https://grafana.example.com/callback"], false),
        );
        templates.insert(
            "mealie".to_string(),
            template(&["https://mealie.example.com/callback"], false),
        );
        let names = vec!["mealie".to_string(), "grafana".to_string()];

        let created = quick_setup(&client(&server), "project-1", &templates, &names)
            .await
            .unwrap();

        grafana.assert_async().await;
        mealie.assert_async().await;
        assert_eq!(created.len(), 2);
        // results align with the requested name order, not template-map order
        assert_eq!(created[0].client_id.as_deref(), Some("cid-mealie"));
        assert_eq!(created[1].client_id.as_deref(), Some("cid-grafana"));
    }

    #[tokio::test]
    async fn quick_setup_errors_on_missing_template() {
        let mut server = Server::new_async().await;
        let create = server
            .mock("POST", "/management/v1/projects/project-1/apps/oidc")
            .expect(0)
            .create_async()
            .await;

        let templates = BTreeMap::new();
        let names = vec!["ghost".to_string()];
        let error = quick_setup(&client(&server), "project-1", &templates, &names)
            .await
            .unwrap_err()
            .to_string();

        create.assert_async().await;
        assert_eq!(error, "template ghost not found");
    }

    #[tokio::test]
    async fn quick_setup_wraps_api_errors_with_template_name() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/management/v1/projects/project-1/apps/oidc")
            .with_status(500)
            .with_header("content-type", "application/json")
            .with_body(r#"{"message":"boom"}"#)
            .create_async()
            .await;

        let mut templates = BTreeMap::new();
        templates.insert(
            "grafana".to_string(),
            template(&["https://grafana.example.com/callback"], false),
        );
        let names = vec!["grafana".to_string()];

        let error = quick_setup(&client(&server), "project-1", &templates, &names)
            .await
            .unwrap_err()
            .to_string();

        assert!(error.starts_with("grafana failed:"), "{error}");
    }
}
