use anyhow::{Context, Result};

use crate::api::{client::ZitadelClient, models::Project};

/// Lists all projects visible to the credential.
pub async fn list(client: &ZitadelClient) -> Result<Vec<Project>> {
    client.list_projects().await
}

/// Id of the instance default project (first search result).
pub async fn default_id(client: &ZitadelClient) -> Result<String> {
    client
        .get_default_project()
        .await?
        .id
        .context("failed to determine default project id")
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    fn client(server: &Server) -> ZitadelClient {
        ZitadelClient::new(server.url(), "test-token".to_string()).unwrap()
    }

    #[tokio::test]
    async fn default_id_returns_first_project_id() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/management/v1/projects/_search")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"result":[{"id":"project-1","name":"Platform"}]}"#)
            .create_async()
            .await;

        let id = default_id(&client(&server)).await.unwrap();

        assert_eq!(id, "project-1");
    }

    #[tokio::test]
    async fn default_id_errors_when_project_lacks_id() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/management/v1/projects/_search")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"result":[{"name":"Platform"}]}"#)
            .create_async()
            .await;

        let error = default_id(&client(&server)).await.unwrap_err().to_string();

        assert_eq!(error, "failed to determine default project id");
    }
}
