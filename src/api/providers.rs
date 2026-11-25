use anyhow::{ensure, Context, Result};
use reqwest::Method;
use serde::Deserialize;
use serde_json::{json, Value};

use super::client::ZitadelClient;

#[cfg(test)]
mod tests;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginSettings {
    #[serde(default)]
    allow_external_idp: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActiveProviders {
    #[serde(default)]
    identity_providers: Vec<ProviderId>,
}

#[derive(Deserialize)]
struct ProviderId {
    id: String,
}

pub(crate) fn owner(provider: &Value) -> Option<&'static str> {
    match provider["owner"].as_str()? {
        "IDP_OWNER_TYPE_SYSTEM" => Some("Instance"),
        "IDP_OWNER_TYPE_ORG" => Some("Organization"),
        _ => None,
    }
}

impl ZitadelClient {
    pub(crate) async fn active_providers(&self) -> Result<Vec<String>> {
        let org = self
            .organization
            .as_deref()
            .context("Select an organization to check sign-in")?;
        let mut url = reqwest::Url::parse("https://localhost/")?;
        url.query_pairs_mut().append_pair("ctx.orgId", org);
        let query = url.query().context("missing organization query")?;
        let settings = self
            .api_request(Method::GET, &format!("/v2/settings/login?{query}"), None)
            .await?;
        let settings: LoginSettings = serde_json::from_value(settings["settings"].clone())
            .context("invalid login settings")?;
        if !settings.allow_external_idp {
            return Ok(Vec::new());
        }
        let active = self
            .api_request(
                Method::GET,
                &format!("/v2/settings/login/idps?{query}"),
                None,
            )
            .await?;
        let active: ActiveProviders =
            serde_json::from_value(active).context("invalid active providers")?;
        Ok(active
            .identity_providers
            .into_iter()
            .map(|provider| provider.id)
            .collect())
    }

    pub(crate) async fn google_provider(&self, id: &str) -> Result<Value> {
        ensure!(
            id.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                && !id.is_empty(),
            "Invalid provider ID"
        );
        let response = self
            .api_request(
                Method::GET,
                &format!("/management/v1/idps/templates/{id}"),
                None,
            )
            .await?;
        let provider = &response["idp"];
        ensure!(
            provider["id"] == id
                && provider
                    .pointer("/config/google/clientId")
                    .and_then(Value::as_str)
                    .is_some(),
            "Selected provider has no Google configuration"
        );
        match owner(provider) {
            Some("Instance") => (),
            Some("Organization")
                if self.organization.is_some()
                    && provider
                        .pointer("/details/resourceOwner")
                        .and_then(Value::as_str)
                        == self.organization.as_deref() => {}
            _ => anyhow::bail!("Cannot verify provider ownership"),
        }
        Ok(provider.clone())
    }

    pub(crate) async fn save_google(
        &self,
        existing: Option<(&str, &str)>,
        scope: &str,
        name: &str,
        client_id: &str,
        secret: &str,
    ) -> Result<()> {
        ensure!(
            !name.trim().is_empty() && !client_id.trim().is_empty(),
            "Name and client ID are required"
        );
        let prefix = match scope {
            "Instance" => "/admin/v1",
            "Organization" if self.organization.is_some() => "/management/v1",
            _ => anyhow::bail!("Choose an instance or selected organization"),
        };
        let mut body = json!({"name":name,"clientId":client_id});
        if !secret.is_empty() {
            body["clientSecret"] = json!(secret);
        }
        let path = format!("{prefix}/idps/google");
        if let Some((id, sequence)) = existing {
            let provider = self.google_provider(id).await?;
            ensure!(
                owner(&provider) == Some(scope)
                    && provider
                        .pointer("/details/sequence")
                        .and_then(Value::as_str)
                        == Some(sequence),
                "Provider changed. Reopen it before editing"
            );
            body["scopes"] = provider
                .pointer("/config/google/scopes")
                .cloned()
                .unwrap_or_else(|| json!([]));
            body["providerOptions"] = provider
                .pointer("/config/options")
                .cloned()
                .unwrap_or_else(|| json!({}));
            self.api_request(Method::PUT, &format!("{path}/{id}"), Some(body))
                .await?;
        } else {
            ensure!(
                !secret.is_empty(),
                "Client secret is required for a new provider"
            );
            let mut client = self.clone();
            if scope == "Instance" {
                client.organization = None;
            }
            client.add_google_idp(client_id, secret, name).await?;
        }
        Ok(())
    }
}
