//! Unified operation layer: exactly one implementation per Zitadel operation,
//! consumed by both the headless `commands/` and the TUI `conductor/`.

pub mod apps;
pub mod idps;
pub mod projects;
pub mod users;

use anyhow::Result;
use serde_json::{json, Value};

use crate::api::{
    client::ZitadelClient,
    models::{App, Idp, User},
};

/// Exact-match selector for application lookup.
#[derive(Debug, Clone)]
pub enum AppSelector {
    Id(String),
    Name(String),
}

/// Exact-match selector for user lookup. `Username` matches either `user_name`
/// or `preferred_login_name`; `Email` matches the human email address.
#[derive(Debug, Clone)]
pub enum UserSelector {
    Id(String),
    Username(String),
    Email(String),
}

/// Exact-match selector for identity provider lookup.
#[derive(Debug, Clone)]
pub enum IdpSelector {
    Id(String),
    Name(String),
}

impl AppSelector {
    fn describe(&self) -> String {
        match self {
            Self::Id(id) => format!("app '{id}'"),
            Self::Name(name) => format!("app '{name}'"),
        }
    }
}

impl UserSelector {
    fn describe(&self) -> String {
        match self {
            Self::Id(id) => format!("user '{id}'"),
            Self::Username(name) => format!("user '{name}'"),
            Self::Email(email) => format!("user '{email}'"),
        }
    }
}

impl IdpSelector {
    fn describe(&self) -> String {
        match self {
            Self::Id(id) => format!("idp '{id}'"),
            Self::Name(name) => format!("idp '{name}'"),
        }
    }
}

/// Typed outcome of a selector resolution. `Ambiguous` carries minimal match
/// info (`{"id": ..., "name": ...}` entries) so callers can surface it in
/// `error.details.matches` without re-shaping (Task 4).
#[derive(Debug)]
pub enum ResolveOutcome<T> {
    One(T),
    NotFound,
    Ambiguous(Vec<Value>),
}

impl<T> ResolveOutcome<T> {
    /// Maps the outcome to a `Result` using the legacy "not found"/"ambiguous"
    /// error strings; `what` describes the looked-up resource, e.g.
    /// `app 'grafana'`. Task 4 replaces this mapping with typed error kinds.
    pub fn into_result(self, what: &str) -> Result<T> {
        match self {
            Self::One(value) => Ok(value),
            Self::NotFound => anyhow::bail!("{what} not found"),
            Self::Ambiguous(matches) => {
                anyhow::bail!("ambiguous {what}: matches {} entries", matches.len())
            }
        }
    }
}

/// Resolves the effective project id: explicit CLI value first, then config,
/// then the instance default project.
pub async fn resolve_project_id(
    client: &ZitadelClient,
    cli_project: Option<String>,
    config_project: Option<String>,
) -> Result<String> {
    if let Some(project) = cli_project.or(config_project) {
        return Ok(project);
    }

    projects::default_id(client).await
}

/// Resolves an app in `project_id` by exact id or name match.
pub async fn resolve_app(
    client: &ZitadelClient,
    project_id: &str,
    selector: &AppSelector,
) -> Result<ResolveOutcome<App>> {
    let apps = client.list_apps(project_id).await?;
    let matches: Vec<App> = apps
        .into_iter()
        .filter(|app| match selector {
            AppSelector::Id(id) => app.id.as_deref() == Some(id.as_str()),
            AppSelector::Name(name) => app.name.as_deref() == Some(name.as_str()),
        })
        .collect();
    Ok(into_outcome(
        matches,
        |app| json!({ "id": app.id, "name": app.name }),
    ))
}

/// Resolves a user by exact id, username (`user_name` or
/// `preferred_login_name`), or human email match.
pub async fn resolve_user(
    client: &ZitadelClient,
    selector: &UserSelector,
) -> Result<ResolveOutcome<User>> {
    let users = client.list_users(USER_LIST_LIMIT).await?;
    let matches: Vec<User> = users
        .into_iter()
        .filter(|user| match selector {
            UserSelector::Id(id) => user.id.as_deref() == Some(id.as_str()),
            UserSelector::Username(name) => {
                user.user_name.as_deref() == Some(name.as_str())
                    || user.preferred_login_name.as_deref() == Some(name.as_str())
            }
            UserSelector::Email(email) => {
                user.human
                    .as_ref()
                    .and_then(|human| human.email.as_ref())
                    .and_then(|human_email| human_email.email.as_deref())
                    == Some(email.as_str())
            }
        })
        .collect();
    Ok(into_outcome(
        matches,
        |user| json!({ "id": user.id, "name": user.user_name }),
    ))
}

/// Resolves an identity provider by exact id or name match.
pub async fn resolve_idp(
    client: &ZitadelClient,
    selector: &IdpSelector,
) -> Result<ResolveOutcome<Idp>> {
    let idps = client.list_idps().await?;
    let matches: Vec<Idp> = idps
        .into_iter()
        .filter(|idp| match selector {
            IdpSelector::Id(id) => idp.id.as_deref() == Some(id.as_str()),
            IdpSelector::Name(name) => idp.name.as_deref() == Some(name.as_str()),
        })
        .collect();
    Ok(into_outcome(
        matches,
        |idp| json!({ "id": idp.id, "name": idp.name }),
    ))
}

const USER_LIST_LIMIT: u64 = 100;

fn into_outcome<T>(mut matches: Vec<T>, describe: impl Fn(&T) -> Value) -> ResolveOutcome<T> {
    match matches.len() {
        0 => ResolveOutcome::NotFound,
        1 => ResolveOutcome::One(matches.pop().expect("one match")),
        _ => ResolveOutcome::Ambiguous(matches.iter().map(describe).collect()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    fn client(server: &Server) -> ZitadelClient {
        ZitadelClient::new(server.url(), "test-token".to_string()).unwrap()
    }

    #[test]
    fn into_result_maps_each_outcome() {
        let one: ResolveOutcome<u32> = ResolveOutcome::One(7);
        assert_eq!(one.into_result("app 'x'").unwrap(), 7);

        let not_found: ResolveOutcome<u32> = ResolveOutcome::NotFound;
        assert_eq!(
            not_found.into_result("app 'x'").unwrap_err().to_string(),
            "app 'x' not found"
        );

        let ambiguous: ResolveOutcome<u32> =
            ResolveOutcome::Ambiguous(vec![json!({"id": "a"}), json!({"id": "b"})]);
        assert_eq!(
            ambiguous.into_result("app 'x'").unwrap_err().to_string(),
            "ambiguous app 'x': matches 2 entries"
        );
    }

    #[tokio::test]
    async fn resolve_project_id_prefers_cli_value() {
        let server = Server::new_async().await;
        let id = resolve_project_id(
            &client(&server),
            Some("cli-project".to_string()),
            Some("config-project".to_string()),
        )
        .await
        .unwrap();
        assert_eq!(id, "cli-project");
    }

    #[tokio::test]
    async fn resolve_project_id_falls_back_to_config() {
        let server = Server::new_async().await;
        let id = resolve_project_id(&client(&server), None, Some("config-project".to_string()))
            .await
            .unwrap();
        assert_eq!(id, "config-project");
    }

    #[tokio::test]
    async fn resolve_project_id_uses_default_project() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/management/v1/projects/_search")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"result":[{"id":"project-1"}]}"#)
            .create_async()
            .await;

        let id = resolve_project_id(&client(&server), None, None)
            .await
            .unwrap();

        mock.assert_async().await;
        assert_eq!(id, "project-1");
    }

    #[tokio::test]
    async fn resolve_project_id_errors_when_default_project_has_no_id() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/management/v1/projects/_search")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"result":[{"name":"no-id"}]}"#)
            .create_async()
            .await;

        let error = resolve_project_id(&client(&server), None, None)
            .await
            .unwrap_err()
            .to_string();

        assert_eq!(error, "failed to determine default project id");
    }

    #[tokio::test]
    async fn resolve_app_by_id_returns_single_match() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/management/v1/projects/project-1/apps/_search")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"result":[{"id":"app-1","name":"grafana"},{"id":"app-2","name":"mealie"}]}"#,
            )
            .create_async()
            .await;

        let outcome = resolve_app(
            &client(&server),
            "project-1",
            &AppSelector::Id("app-2".to_string()),
        )
        .await
        .unwrap();

        match outcome {
            ResolveOutcome::One(app) => assert_eq!(app.id.as_deref(), Some("app-2")),
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn resolve_app_by_name_matches_exactly() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/management/v1/projects/project-1/apps/_search")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"result":[{"id":"app-1","name":"grafana-2"}]}"#)
            .create_async()
            .await;

        let outcome = resolve_app(
            &client(&server),
            "project-1",
            &AppSelector::Name("grafana".to_string()),
        )
        .await
        .unwrap();

        assert!(matches!(outcome, ResolveOutcome::NotFound));
    }

    #[tokio::test]
    async fn resolve_app_ambiguous_reports_match_summaries() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/management/v1/projects/project-1/apps/_search")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"result":[{"id":"app-1","name":"grafana"},{"id":"app-2","name":"grafana"}]}"#,
            )
            .create_async()
            .await;

        let outcome = resolve_app(
            &client(&server),
            "project-1",
            &AppSelector::Name("grafana".to_string()),
        )
        .await
        .unwrap();

        match outcome {
            ResolveOutcome::Ambiguous(matches) => assert_eq!(
                matches,
                vec![
                    json!({"id": "app-1", "name": "grafana"}),
                    json!({"id": "app-2", "name": "grafana"}),
                ]
            ),
            other => panic!("expected Ambiguous, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn resolve_user_username_matches_user_name_and_login_name() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/management/v1/users/_search")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"result":[
                    {"id":"user-1","userName":"admin","preferredLoginName":"admin@zitadel.example.com"},
                    {"id":"user-2","userName":"other","preferredLoginName":"alice@example.com"}
                ]}"#,
            )
            .expect(2)
            .create_async()
            .await;

        let by_user_name = resolve_user(
            &client(&server),
            &UserSelector::Username("admin".to_string()),
        )
        .await
        .unwrap();
        let by_login_name = resolve_user(
            &client(&server),
            &UserSelector::Username("alice@example.com".to_string()),
        )
        .await
        .unwrap();

        mock.assert_async().await;
        match by_user_name {
            ResolveOutcome::One(user) => assert_eq!(user.id.as_deref(), Some("user-1")),
            other => panic!("expected One, got {other:?}"),
        }
        match by_login_name {
            ResolveOutcome::One(user) => assert_eq!(user.id.as_deref(), Some("user-2")),
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn resolve_user_by_email_matches_human_email() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/management/v1/users/_search")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"result":[
                    {"id":"user-1","userName":"admin","human":{"email":{"email":"admin@example.com"}}},
                    {"id":"user-2","userName":"sa","machine":{"name":"sa"}}
                ]}"#,
            )
            .create_async()
            .await;

        let outcome = resolve_user(
            &client(&server),
            &UserSelector::Email("admin@example.com".to_string()),
        )
        .await
        .unwrap();

        match outcome {
            ResolveOutcome::One(user) => assert_eq!(user.id.as_deref(), Some("user-1")),
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn resolve_user_not_found_and_ambiguous() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/management/v1/users/_search")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"result":[
                    {"id":"user-1","userName":"dup"},
                    {"id":"user-2","userName":"dup"}
                ]}"#,
            )
            .expect(2)
            .create_async()
            .await;

        let missing = resolve_user(
            &client(&server),
            &UserSelector::Username("ghost".to_string()),
        )
        .await
        .unwrap();
        let dup = resolve_user(&client(&server), &UserSelector::Id("user-1".to_string()))
            .await
            .unwrap();

        assert!(matches!(missing, ResolveOutcome::NotFound));
        match dup {
            ResolveOutcome::One(user) => assert_eq!(user.user_name.as_deref(), Some("dup")),
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn resolve_idp_by_name_and_ambiguous() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/admin/v1/idps/_search")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"result":[
                    {"id":"idp-1","name":"Google"},
                    {"id":"idp-2","name":"Google"},
                    {"id":"idp-3","name":"GitHub"}
                ]}"#,
            )
            .expect(2)
            .create_async()
            .await;

        let single = resolve_idp(&client(&server), &IdpSelector::Id("idp-3".to_string()))
            .await
            .unwrap();
        let ambiguous = resolve_idp(&client(&server), &IdpSelector::Name("Google".to_string()))
            .await
            .unwrap();

        match single {
            ResolveOutcome::One(idp) => assert_eq!(idp.name.as_deref(), Some("GitHub")),
            other => panic!("expected One, got {other:?}"),
        }
        match ambiguous {
            ResolveOutcome::Ambiguous(matches) => assert_eq!(matches.len(), 2),
            other => panic!("expected Ambiguous, got {other:?}"),
        }
    }
}
