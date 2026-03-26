use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, bail, Context, Result};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

#[derive(Debug, Clone)]
pub struct ResolvedAuth {
    pub token: String,
    pub source: &'static str,
}

#[derive(Debug, Deserialize)]
struct ServiceAccountKey {
    #[serde(rename = "keyId")]
    key_id: String,
    #[serde(rename = "userId")]
    user_id: String,
    key: String,
}

#[derive(Debug, Serialize)]
struct Claims {
    iss: String,
    sub: String,
    aud: String,
    iat: usize,
    exp: usize,
}

pub async fn resolve_access_token(
    client: &Client,
    zitadel_url: &str,
    cli_pat: Option<String>,
    cli_service_account_file: Option<PathBuf>,
    config: &AppConfig,
) -> Result<ResolvedAuth> {
    if let Some(pat) = cli_pat {
        return Ok(ResolvedAuth {
            token: pat,
            source: "cli PAT",
        });
    }

    if let Ok(pat) = std::env::var("ZITADEL_TOKEN") {
        if !pat.trim().is_empty() {
            return Ok(ResolvedAuth {
                token: pat,
                source: "env PAT",
            });
        }
    }

    if let Some(pat) = config.pat.clone() {
        return Ok(ResolvedAuth {
            token: pat,
            source: "config PAT",
        });
    }

    if let Some(path) = cli_service_account_file {
        let token = exchange_service_account(client, zitadel_url, path).await?;
        return Ok(ResolvedAuth {
            token,
            source: "cli service account",
        });
    }

    if let Ok(path) = std::env::var("ZITADEL_SERVICE_ACCOUNT_FILE") {
        let token = exchange_service_account(client, zitadel_url, PathBuf::from(path)).await?;
        return Ok(ResolvedAuth {
            token,
            source: "env service account",
        });
    }

    if let Some(path) = config.service_account_file.clone() {
        let token = exchange_service_account(client, zitadel_url, path).await?;
        return Ok(ResolvedAuth {
            token,
            source: "config service account",
        });
    }

    bail!("no credentials available; use --token, --service-account-file, env vars, or config")
}

async fn exchange_service_account(
    client: &Client,
    zitadel_url: &str,
    path: PathBuf,
) -> Result<String> {
    let contents =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let key: ServiceAccountKey = serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    let jwt = create_jwt(&key, zitadel_url)?;

    let token_url = format!("{}/oauth/v2/token", zitadel_url.trim_end_matches('/'));
    let params = [
        ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
        ("scope", "openid urn:zitadel:iam:org:project:id:zitadel:aud"),
        ("assertion", jwt.as_str()),
    ];

    let response = client.post(token_url).form(&params).send().await?;
    if !response.status().is_success() {
        bail!(
            "service-account token exchange failed: {}",
            response.text().await?
        );
    }

    let body: serde_json::Value = response.json().await?;
    body.get("access_token")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("token response missing access_token"))
}

fn create_jwt(key: &ServiceAccountKey, audience: &str) -> Result<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock before unix epoch")?
        .as_secs() as usize;

    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(key.key_id.clone());

    let claims = Claims {
        iss: key.user_id.clone(),
        sub: key.user_id.clone(),
        aud: audience.to_string(),
        iat: now,
        exp: now + 3600,
    };

    encode(
        &header,
        &claims,
        &EncodingKey::from_rsa_pem(key.key.as_bytes()).context("invalid RSA private key")?,
    )
    .context("failed to sign JWT")
}
