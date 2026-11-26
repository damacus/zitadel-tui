use anyhow::Result;

use crate::{
    api::{
        client::ZitadelClient,
        models::{CreateUserResult, MemberResult, User},
    },
    config::UserTemplate,
};

use super::{resolve_user, UserSelector};

/// Lists users, up to `limit`.
pub async fn list(client: &ZitadelClient, limit: u64) -> Result<Vec<User>> {
    client.list_users(limit).await
}

/// Looks up a single user by selector, erroring on zero or multiple matches.
pub async fn get(client: &ZitadelClient, selector: &UserSelector) -> Result<User> {
    resolve_user(client, selector)
        .await?
        .into_result(&selector.describe())
}

/// Creates a human user via the v2 users API.
pub async fn create_human(
    client: &ZitadelClient,
    email: &str,
    first_name: &str,
    last_name: &str,
    username: Option<&str>,
) -> Result<CreateUserResult> {
    client
        .create_human_user(email, first_name, last_name, username)
        .await
}

/// Imports a human user with a password via the management API.
pub async fn import_admin(
    client: &ZitadelClient,
    username: &str,
    first_name: &str,
    last_name: &str,
    email: &str,
    password: &str,
    password_change_required: bool,
) -> Result<CreateUserResult> {
    client
        .import_human_user(
            username,
            first_name,
            last_name,
            email,
            password,
            password_change_required,
        )
        .await
}

/// Grants the IAM_OWNER membership to a user.
pub async fn grant_iam_owner(client: &ZitadelClient, user_id: &str) -> Result<MemberResult> {
    client.grant_iam_owner(user_id).await
}

/// One entry of [`quick_setup`]: the created user plus whether an IAM_OWNER
/// grant was applied (drives the conductor's summary lines).
#[derive(Debug)]
pub struct QuickSetupUserResult {
    pub email: String,
    pub result: CreateUserResult,
    pub iam_owner_granted: bool,
}

/// Creates the template users concurrently, granting IAM_OWNER to `admin`
/// entries. Results are ordered by input index.
pub async fn quick_setup(
    client: &ZitadelClient,
    users: Vec<UserTemplate>,
) -> Result<Vec<QuickSetupUserResult>> {
    let mut join_set = tokio::task::JoinSet::new();

    for (index, user) in users.into_iter().enumerate() {
        let client = client.clone();
        join_set.spawn(async move {
            let outcome = async {
                let result = client
                    .create_human_user(&user.email, &user.first_name, &user.last_name, None)
                    .await?;
                let mut iam_owner_granted = false;
                if user.admin {
                    if let Some(user_id) = result.user_id.as_deref() {
                        client.grant_iam_owner(user_id).await?;
                        iam_owner_granted = true;
                    }
                }
                Ok::<_, anyhow::Error>(QuickSetupUserResult {
                    email: user.email.clone(),
                    result,
                    iam_owner_granted,
                })
            }
            .await;
            (index, user.email.clone(), outcome)
        });
    }

    let mut results = Vec::new();
    while let Some(joined) = join_set.join_next().await {
        let (index, email, outcome) = joined?;
        let outcome =
            outcome.map_err(|error| anyhow::anyhow!("Failed to create {email}: {error}"))?;
        results.push((index, outcome));
    }
    results.sort_by_key(|(index, _)| *index);

    Ok(results.into_iter().map(|(_, outcome)| outcome).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server};
    use serde_json::json;

    fn client(server: &Server) -> ZitadelClient {
        ZitadelClient::new(server.url(), "test-token".to_string()).unwrap()
    }

    fn user_template(email: &str, admin: bool) -> UserTemplate {
        UserTemplate {
            email: email.to_string(),
            first_name: "First".to_string(),
            last_name: "Last".to_string(),
            admin,
        }
    }

    #[tokio::test]
    async fn get_resolves_user_by_email() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/management/v1/users/_search")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"result":[{"id":"user-1","userName":"admin","human":{"email":{"email":"admin@example.com"}}}]}"#,
            )
            .create_async()
            .await;

        let user = get(
            &client(&server),
            &UserSelector::Email("admin@example.com".to_string()),
        )
        .await
        .unwrap();

        assert_eq!(user.id.as_deref(), Some("user-1"));
    }

    #[tokio::test]
    async fn create_human_passes_through_client_result() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v2/users/human")
            .match_header("authorization", "Bearer test-token")
            .match_body(Matcher::PartialJson(json!({
                "username": "alice",
                "email": {"email": "alice@example.com"}
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"userId":"user-1"}"#)
            .create_async()
            .await;

        let created = create_human(
            &client(&server),
            "alice@example.com",
            "Alice",
            "Admin",
            Some("alice"),
        )
        .await
        .unwrap();

        mock.assert_async().await;
        assert_eq!(created.user_id.as_deref(), Some("user-1"));
    }

    #[tokio::test]
    async fn grant_iam_owner_posts_membership() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/admin/v1/members")
            .match_body(Matcher::PartialJson(json!({
                "userId": "user-1",
                "roles": ["IAM_OWNER"]
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"memberId":"member-1"}"#)
            .create_async()
            .await;

        let member = grant_iam_owner(&client(&server), "user-1").await.unwrap();

        mock.assert_async().await;
        assert_eq!(member.member_id.as_deref(), Some("member-1"));
    }

    #[tokio::test]
    async fn quick_setup_creates_users_and_grants_admins() {
        let mut server = Server::new_async().await;
        let create_admin = server
            .mock("POST", "/v2/users/human")
            .match_body(Matcher::PartialJson(json!({
                "email": {"email": "admin@example.com"}
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"userId":"user-admin"}"#)
            .create_async()
            .await;
        let create_user = server
            .mock("POST", "/v2/users/human")
            .match_body(Matcher::PartialJson(json!({
                "email": {"email": "user@example.com"}
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"userId":"user-regular"}"#)
            .create_async()
            .await;
        let grant = server
            .mock("POST", "/admin/v1/members")
            .match_body(Matcher::PartialJson(json!({
                "userId": "user-admin",
                "roles": ["IAM_OWNER"]
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"memberId":"member-1"}"#)
            .create_async()
            .await;

        let users = vec![
            user_template("admin@example.com", true),
            user_template("user@example.com", false),
        ];
        let results = quick_setup(&client(&server), users).await.unwrap();

        create_admin.assert_async().await;
        create_user.assert_async().await;
        grant.assert_async().await;
        assert_eq!(results.len(), 2);
        // ordered by input index
        assert_eq!(results[0].email, "admin@example.com");
        assert!(results[0].iam_owner_granted);
        assert_eq!(results[0].result.user_id.as_deref(), Some("user-admin"));
        assert_eq!(results[1].email, "user@example.com");
        assert!(!results[1].iam_owner_granted);
    }

    #[tokio::test]
    async fn quick_setup_admin_without_user_id_skips_grant() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/v2/users/human")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .create_async()
            .await;
        let grant = server
            .mock("POST", "/admin/v1/members")
            .expect(0)
            .create_async()
            .await;

        let users = vec![user_template("admin@example.com", true)];
        let results = quick_setup(&client(&server), users).await.unwrap();

        grant.assert_async().await;
        assert_eq!(results.len(), 1);
        assert!(!results[0].iam_owner_granted);
    }

    #[tokio::test]
    async fn quick_setup_wraps_errors_with_user_email() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/v2/users/human")
            .with_status(500)
            .with_header("content-type", "application/json")
            .with_body(r#"{"message":"boom"}"#)
            .create_async()
            .await;

        let users = vec![user_template("admin@example.com", false)];
        let error = quick_setup(&client(&server), users)
            .await
            .unwrap_err()
            .to_string();

        assert!(
            error.starts_with("Failed to create admin@example.com:"),
            "{error}"
        );
    }
}
