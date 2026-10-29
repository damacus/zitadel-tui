use anyhow::Result;

use crate::api::{client::ZitadelClient, models::Idp};

use super::{resolve_idp, IdpSelector};

/// Lists configured identity providers, up to `limit`.
pub async fn list(client: &ZitadelClient, limit: u64) -> Result<Vec<Idp>> {
    client.list_idps(limit).await
}

/// Looks up a single identity provider by selector, erroring on zero or
/// multiple matches.
pub async fn get(client: &ZitadelClient, selector: &IdpSelector) -> Result<Idp> {
    resolve_idp(client, selector)
        .await?
        .into_result(&selector.describe())
}

/// Creates the Google identity provider with manual OAuth credentials.
pub async fn configure_google(
    client: &ZitadelClient,
    client_id: &str,
    client_secret: &str,
    name: &str,
) -> Result<Idp> {
    client.add_google_idp(client_id, client_secret, name).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server};
    use serde_json::json;

    fn client(server: &Server) -> ZitadelClient {
        ZitadelClient::new(server.url(), "test-token".to_string()).unwrap()
    }

    #[tokio::test]
    async fn configure_google_posts_google_idp() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/admin/v1/idps/google")
            .match_header("authorization", "Bearer test-token")
            .match_body(Matcher::PartialJson(json!({
                "name": "Google",
                "clientId": "google-client-id",
                "clientSecret": "google-secret"
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"idp-1","name":"Google"}"#)
            .create_async()
            .await;

        let idp = configure_google(
            &client(&server),
            "google-client-id",
            "google-secret",
            "Google",
        )
        .await
        .unwrap();

        mock.assert_async().await;
        assert_eq!(idp.id.as_deref(), Some("idp-1"));
    }

    #[tokio::test]
    async fn get_errors_with_ambiguous_text() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/admin/v1/idps/_search")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"result":[{"id":"idp-1","name":"Google"},{"id":"idp-2","name":"Google"}]}"#,
            )
            .create_async()
            .await;

        let error = get(&client(&server), &IdpSelector::Name("Google".to_string()))
            .await
            .unwrap_err()
            .to_string();

        assert_eq!(error, "ambiguous idp 'Google': matches 2 entries");
    }
}
