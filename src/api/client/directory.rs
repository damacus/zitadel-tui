use super::*;
use std::collections::HashSet;

impl ZitadelClient {
    pub(crate) async fn directory_pages(&self, path: &str, mut body: Value) -> Result<Vec<Value>> {
        let mut rows = Vec::new();
        let mut seen = HashSet::new();
        loop {
            body["query"] = json!({"limit": 100, "offset": rows.len().to_string(), "asc": true});
            let response = self
                .api_request(Method::POST, path, Some(body.clone()))
                .await?;
            let page = match response.get("result") {
                Some(value) => value
                    .as_array()
                    .context("invalid directory search response")?
                    .as_slice(),
                None => &[],
            };
            let total = response
                .pointer("/details/totalResult")
                .and_then(|n| n.as_u64().or_else(|| n.as_str()?.parse().ok()));
            for row in page {
                let id = row["id"]
                    .as_str()
                    .or_else(|| row["userId"].as_str())
                    .or_else(|| row["key"].as_str())
                    .or_else(|| row["grantId"].as_str())
                    .context("directory entry is missing an ID")?;
                if !seen.insert(id.to_owned()) {
                    bail!("directory changed during pagination; refresh to retry");
                }
                rows.push(row.clone());
            }
            if page.is_empty() && total.is_some_and(|count| count > rows.len() as u64) {
                bail!("directory changed during pagination; refresh to retry");
            }
            if page.is_empty() || total.map_or(page.len() < 100, |count| rows.len() as u64 >= count)
            {
                return Ok(rows);
            }
        }
    }

    pub(crate) async fn directory_organizations(&self) -> Result<Vec<(String, String)>> {
        self.directory_choices(
            "/v2/organizations/_search",
            json!({"sortingColumn":"ORGANIZATION_FIELD_NAME_NAME"}),
        )
        .await
    }

    pub(crate) async fn directory_projects(&self) -> Result<Vec<(String, String)>> {
        self.directory_choices("/management/v1/projects/_search", json!({}))
            .await
    }

    pub(crate) async fn directory_granted_projects(&self) -> Result<Vec<GrantedProject>> {
        let rows = self
            .directory_pages("/management/v1/granted_projects/_search", json!({}))
            .await?;
        rows.into_iter()
            .filter(|row| row["state"] == "PROJECT_GRANT_STATE_ACTIVE")
            .map(|row| {
                if let Some(org) = self.organization.as_deref() {
                    anyhow::ensure!(
                        row["grantedOrgId"].as_str() == Some(org),
                        "Project grant belongs to another organization"
                    );
                }
                Ok(serde_json::from_value(row)?)
            })
            .collect()
    }

    async fn directory_choices(&self, path: &str, body: Value) -> Result<Vec<(String, String)>> {
        self.directory_pages(path, body)
            .await?
            .into_iter()
            .map(|row| {
                let id = row["id"].as_str().context("missing resource ID")?;
                Ok((id.to_owned(), row["name"].as_str().unwrap_or(id).to_owned()))
            })
            .collect()
    }

    pub(crate) async fn directory_apps(&self, project: &str) -> Result<Vec<App>> {
        self.directory_pages(
            &format!("/management/v1/projects/{project}/apps/_search"),
            json!({}),
        )
        .await?
        .into_iter()
        .map(decode)
        .collect()
    }

    pub(crate) async fn directory_users(&self) -> Result<Vec<User>> {
        self.directory_pages("/management/v1/users/_search", json!({}))
            .await?
            .into_iter()
            .map(decode)
            .collect()
    }

    pub(crate) async fn directory_idps(&self) -> Result<Vec<Idp>> {
        self.directory_pages("/management/v1/idps/templates/_search", json!({}))
            .await?
            .into_iter()
            .map(decode)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server};

    #[tokio::test]
    async fn selected_organization_is_sent_on_every_page_and_on_user_creation() {
        let mut server = Server::new_async().await;
        let mut client = ZitadelClient::new(server.url(), "token".into()).unwrap();
        client.organization = Some("org-2".into());
        let mut mocks = vec![];
        for (offset, id) in [(0, "alice"), (1, "bob")] {
            mocks.push(
                server
                    .mock("POST", "/management/v1/users/_search")
                    .match_header("x-zitadel-orgid", "org-2")
                    .match_body(Matcher::PartialJson(
                        json!({"query":{"offset":offset.to_string()}}),
                    ))
                    .with_status(200)
                    .with_body(
                        json!({"details":{"totalResult":"2"},"result":[{"id":id}]}).to_string(),
                    )
                    .create_async()
                    .await,
            );
        }
        let users = client.directory_users().await.unwrap();
        assert_eq!(users.len(), 2);
        assert_eq!(users[1].id.as_deref(), Some("bob"));
        let create = server
            .mock("POST", "/v2/users/human")
            .match_body(Matcher::PartialJson(
                json!({"organization":{"orgId":"org-2"}}),
            ))
            .with_status(200)
            .with_body("{}")
            .create_async()
            .await;
        client
            .create_human_user("new@example.org", "New", "User", None)
            .await
            .unwrap();
        for mock in mocks {
            mock.assert_async().await;
        }
        create.assert_async().await;
    }
}
