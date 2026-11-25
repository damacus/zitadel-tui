use anyhow::{ensure, Context, Result};
use reqwest::Method;
use serde::Deserialize;
use serde_json::{json, Value};

use super::client::ZitadelClient;

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server};

    #[tokio::test]
    async fn revoke_preserves_other_roles_and_refuses_stale_review() {
        let mut server = Server::new_async().await;
        let client = ZitadelClient::new(server.url(), "token".into()).unwrap();
        let search = server.mock("POST", "/management/v1/users/grants/_search")
            .match_body(Matcher::PartialJson(json!({"queries":[{"userIdQuery":{"userId":"u"}},{"projectIdQuery":{"projectId":"p"}}]})))
            .with_body(r#"{"result":[{"id":"g","userId":"u","projectId":"p","roleKeys":["reader","writer"]}]}"#)
            .expect(3).create_async().await;
        let put = server
            .mock("PUT", "/management/v1/users/u/grants/g")
            .match_body(Matcher::Json(json!({"roleKeys":["reader"]})))
            .with_status(200)
            .create_async()
            .await;
        let mut review = client.user_project_grant("u", "p").await.unwrap().unwrap();
        client
            .change_user_role("u", "p", Some(&review), "writer", false)
            .await
            .unwrap();
        review.role_keys.pop();
        assert!(client
            .change_user_role("u", "p", Some(&review), "reader", false)
            .await
            .is_err());
        search.assert_async().await;
        put.assert_async().await;
    }

    #[tokio::test]
    async fn final_role_removes_grant_and_first_role_creates_one() {
        let mut server = Server::new_async().await;
        let client = ZitadelClient::new(server.url(), "token".into()).unwrap();
        let empty = server
            .mock("POST", "/management/v1/users/grants/_search")
            .with_body("{}")
            .create_async()
            .await;
        let create = server
            .mock("POST", "/management/v1/users/u/grants")
            .match_body(Matcher::Json(
                json!({"projectId":"p","roleKeys":["reader"]}),
            ))
            .create_async()
            .await;
        client
            .change_user_role("u", "p", None, "reader", true)
            .await
            .unwrap();
        create.assert_async().await;
        empty.remove_async().await;
        let grant = json!({"id":"g","userId":"u","projectId":"p","roleKeys":["reader"]});
        let _search = server
            .mock("POST", "/management/v1/users/grants/_search")
            .with_body(json!({"result":[grant]}).to_string())
            .create_async()
            .await;
        let remove = server
            .mock("DELETE", "/management/v1/users/u/grants/g")
            .create_async()
            .await;
        let reviewed = serde_json::from_value(grant).unwrap();
        client
            .change_user_role("u", "p", Some(&reviewed), "reader", false)
            .await
            .unwrap();
        remove.assert_async().await;
    }

    #[tokio::test]
    async fn lifecycle_requests_target_selected_user_and_org() {
        let mut server = Server::new_async().await;
        let mut client = ZitadelClient::new(server.url(), "token".into()).unwrap();
        client.organization = Some("org".into());
        for (operation, unlock) in [("_unlock", true), ("_deactivate", false)] {
            let path = format!("/management/v1/users/u/{operation}");
            let request = server
                .mock("POST", path.as_str())
                .match_header("x-zitadel-orgid", "org")
                .create_async()
                .await;
            client.change_user_state("u", unlock).await.unwrap();
            request.assert_async().await;
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Grant {
    pub id: String,
    pub user_id: String,
    pub project_id: String,
    #[serde(default)]
    pub role_keys: Vec<String>,
}

impl ZitadelClient {
    pub(crate) async fn user_project_grant(
        &self,
        user: &str,
        project: &str,
    ) -> Result<Option<Grant>> {
        let response = self.api_request(Method::POST, "/management/v1/users/grants/_search", Some(json!({
            "query": {"limit": 2},
            "queries": [{"userIdQuery":{"userId":user}}, {"projectIdQuery":{"projectId":project}}]
        }))).await?;
        let rows = response.get("result").cloned().unwrap_or(json!([]));
        if let Some(org) = self.organization.as_ref() {
            ensure!(
                rows.as_array()
                    .context("Invalid grants result")?
                    .iter()
                    .all(|row| row
                        .pointer("/details/resourceOwner")
                        .and_then(Value::as_str)
                        == Some(org.as_str())),
                "Returned authorization belongs to another organization."
            );
        }
        let mut grants: Vec<Grant> = serde_json::from_value(rows)?;
        ensure!(
            grants.len() <= 1,
            "Several authorizations match this user and project."
        );
        if let Some(grant) = grants.first() {
            ensure!(
                grant.user_id == user && grant.project_id == project && !grant.id.is_empty(),
                "Authorization target does not match the selection."
            );
        }
        Ok(grants.pop())
    }

    pub(crate) async fn project_role_keys(&self, project: &str) -> Result<Vec<String>> {
        let path = format!("/management/v1/projects/{project}/roles/_search");
        self.directory_pages(&path, json!({}))
            .await?
            .into_iter()
            .map(|row| {
                row["key"]
                    .as_str()
                    .map(str::to_owned)
                    .context("Project role has no key")
            })
            .collect()
    }

    pub(crate) async fn change_user_role(
        &self,
        user: &str,
        project: &str,
        reviewed: Option<&Grant>,
        role: &str,
        add: bool,
    ) -> Result<()> {
        let current = self.user_project_grant(user, project).await?;
        ensure!(
            current.as_ref() == reviewed,
            "Roles changed since review. Open the action again."
        );
        let path = format!("/management/v1/users/{user}/grants");
        let (method, path, body) = if let Some(grant) = current {
            let path = format!("{path}/{}", grant.id);
            let mut keys = grant.role_keys;
            ensure!(
                keys.iter().any(|key| key == role) != add,
                "The selected role is already in the requested state."
            );
            if add {
                keys.push(role.to_owned());
            } else {
                keys.retain(|key| key != role);
            }
            if keys.is_empty() {
                (Method::DELETE, path, None)
            } else {
                (Method::PUT, path, Some(json!({"roleKeys":keys})))
            }
        } else {
            ensure!(add, "There is no role to revoke.");
            (
                Method::POST,
                path,
                Some(json!({"projectId":project,"roleKeys":[role]})),
            )
        };
        self.api_request(method, &path, body).await?;
        Ok(())
    }

    pub(crate) async fn change_user_state(&self, user: &str, unlock: bool) -> Result<()> {
        let operation = if unlock { "_unlock" } else { "_deactivate" };
        self.api_request(
            Method::POST,
            &format!("/management/v1/users/{user}/{operation}"),
            None,
        )
        .await?;
        Ok(())
    }
}
