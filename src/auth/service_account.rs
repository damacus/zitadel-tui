use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context, Result};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::Client;
use serde::{Deserialize, Serialize};

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

pub(crate) async fn exchange_service_account(
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
    let status = response.status();
    let body = response.bytes().await?;
    if !status.is_success() {
        anyhow::bail!("service-account token exchange failed ({status})");
    }

    let body: serde_json::Value =
        serde_json::from_slice(&body).context("failed to decode service-account token response")?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use std::{env, fs};

    fn temp_file(name: &str, contents: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = env::temp_dir().join(format!("zitadel-tui-{name}-{unique}.json"));
        fs::write(&path, contents).unwrap();
        path
    }

    const TEST_PRIVATE_KEY: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQCqgMTyfrfQTZd/
wndpcfCBDfG919Zi+Lz6H3MI48BuOT4T0MeTkjRMyY+gSWcOxwvtlMsQoB41xHeW
/fVrFKqgbsEZRoH5ZxAoN7+pNnfEfCXBhtLbs2Oi6Gt+TTCE9utv2qoXUkHCys82
Y4kZqddDo3AYm4lHXgyyfadiPoXiZx38XeiD2BLNIKglBhCrRLA2CU5l3rFhW32F
9rAyfGg00pQoehOLdtfJ0u1ZDH6rXa+FVIW5MBhCpUR4LvIePh3gT2W85QrRuTV2
rYgu6OlAIUsQODEvZx89KrqGMFVTe41Y1icXbUerebRhfQtgvR6c5LErkpoWfXbq
bhJjnqodAgMBAAECggEAHZ2bWAGhvPlVSNhG9JZZb5kz9cVBVFSfQpTm3tLskFi6
CygXGm9pTTMvkuhdEciKLlzLftpJFQ7ItP3svIpM7uv9931zQxZfTJUOYf53hDYK
OtjH1GiO9HOJhFk1Bct77qRdKgrcKFEg9/IHFOGW5gVECcouaKqR6wj/Y4W3rLDy
9ijNJBDqtWRWjsHsba8q9dNa6UMTqoCXSrZ5AfEGMbR9wmbTPRTip8meiZzZdgRK
51LTr7bFPJFRFm8g96lbWRsIn4NKdpSW33jzvlxedGIJLV4LMka1oBTyYM4tWVMs
4aA1/PtfouCBm98wYm2SPU019D5E9v2M6lphYkYREwKBgQDZV42qh5lfYCL+S6vc
n1kFYC58p3+/Mi84LXZYoORvCH5EGLwv0ftM2uT0amlv3Lcic23hf3SVtGRobo5n
wssT6P/LXiP9dEpcDKziAXjouXvm4fSDUvEgmXlaGfQmR2LfEIug/qusvZQq/F5p
hKb2xuRfQzitsZpxA4Pot5DcawKBgQDI1HMnmc+FmRzVfHaXUgnD406heBJOYtie
GgtW9qVW/aZf8N/G36eDlj0/u4uRFlPcp6Ad427eoMWHLqCcMgpXtdZ5SckMJ0pR
RCKxMJ37/SOhV0R8v/sp5q9gRVy1z4iomON0oUElqukuIJzG3Lp86dwhQR7OpZ7C
wMGIGT+1lwKBgQDRJbnD8n0bFM5X28Xkpsrpq2bQufbqrIZYDxelrh5k4s1vBkaB
1hV4HeTZd1VDOihZVK7Wouoz7cX54PnUy9TUshEFSPBlRHUSI3hyfGw3t9aNlb7Q
aQ51CnuGwxb5hxSUB732DVxy/HQK8ZSBAhARxc+aBHwUWaZ/Ppy/Y3ZZRQKBgQC1
dcDW91NbE43KGDvPXoEUTj6uByADU60GreGxIgsjHu1Fow+PUma5rvaIr5zb66C6
r3sthmKXJg1Up/zXJR/TQKoZzWGraZTs+POfxp35IjEfhwqK7ayzn4y3H/U1EeAY
9owOxeVnc5Zd53nA9ZBLbcNJCN4dOejJcAFuR+IY4QKBgEEIjBV+z/Nz6DInO8OD
zARHgiLinSZh85rgNytB9UYXvbrOwKwd7hQDbxXLKqCx1f52Y1TdsWq7H3ST8+YO
dfy4Xqt23aNLgPpM7pQa8J88yq7+YrwlGynnTiDUNoHQqmzDHjKlWIT4bQONX5aO
ehuWWRLZbrtEDcwsUeaYjDGj
-----END PRIVATE KEY-----
"#;

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn exchanges_service_account_and_returns_access_token() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/oauth/v2/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"access_token":"jwt-exchanged-token"}"#)
            .create_async()
            .await;

        let _guard = crate::test_support::env_lock();
        let original_token = env::var("ZITADEL_TOKEN").ok();
        let original_service_account = env::var("ZITADEL_SERVICE_ACCOUNT_FILE").ok();
        env::remove_var("ZITADEL_TOKEN");
        env::remove_var("ZITADEL_SERVICE_ACCOUNT_FILE");
        let server_url = server.url();

        let key_path = temp_file(
            "service-account",
            &serde_json::json!({
                "keyId": "kid-1",
                "userId": "user-1",
                "key": TEST_PRIVATE_KEY
            })
            .to_string(),
        );
        let http = Client::new();
        let token = exchange_service_account(&http, &server_url, key_path)
            .await
            .unwrap();

        if let Some(value) = original_token {
            env::set_var("ZITADEL_TOKEN", value);
        }
        if let Some(value) = original_service_account {
            env::set_var("ZITADEL_SERVICE_ACCOUNT_FILE", value);
        }

        mock.assert_async().await;
        assert_eq!(token, "jwt-exchanged-token");
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn missing_access_token_is_rejected() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/oauth/v2/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .create_async()
            .await;

        let _guard = crate::test_support::env_lock();
        let original_token = env::var("ZITADEL_TOKEN").ok();
        let original_service_account = env::var("ZITADEL_SERVICE_ACCOUNT_FILE").ok();
        env::remove_var("ZITADEL_TOKEN");
        env::remove_var("ZITADEL_SERVICE_ACCOUNT_FILE");
        let server_url = server.url();

        let key_path = temp_file(
            "service-account-missing-token",
            &serde_json::json!({
                "keyId": "kid-1",
                "userId": "user-1",
                "key": TEST_PRIVATE_KEY
            })
            .to_string(),
        );
        let http = Client::new();
        let error = exchange_service_account(&http, &server_url, key_path)
            .await
            .unwrap_err()
            .to_string();

        if let Some(value) = original_token {
            env::set_var("ZITADEL_TOKEN", value);
        }
        if let Some(value) = original_service_account {
            env::set_var("ZITADEL_SERVICE_ACCOUNT_FILE", value);
        }

        mock.assert_async().await;
        assert!(error.contains("missing access_token"));
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn token_exchange_errors_do_not_echo_response_body() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/oauth/v2/token")
            .with_status(500)
            .with_header("content-type", "application/json")
            .with_body(r#"{"client_secret":"leaked"}"#)
            .create_async()
            .await;

        let _guard = crate::test_support::env_lock();
        let original_token = env::var("ZITADEL_TOKEN").ok();
        let original_service_account = env::var("ZITADEL_SERVICE_ACCOUNT_FILE").ok();
        env::remove_var("ZITADEL_TOKEN");
        env::remove_var("ZITADEL_SERVICE_ACCOUNT_FILE");
        let server_url = server.url();

        let key_path = temp_file(
            "service-account-error",
            &serde_json::json!({
                "keyId": "kid-1",
                "userId": "user-1",
                "key": TEST_PRIVATE_KEY
            })
            .to_string(),
        );
        let http = Client::new();
        let error = exchange_service_account(&http, &server_url, key_path)
            .await
            .unwrap_err()
            .to_string();

        if let Some(value) = original_token {
            env::set_var("ZITADEL_TOKEN", value);
        }
        if let Some(value) = original_service_account {
            env::set_var("ZITADEL_SERVICE_ACCOUNT_FILE", value);
        }

        mock.assert_async().await;
        assert!(error.contains("service-account token exchange failed"));
        assert!(!error.contains("leaked"));
    }
}
