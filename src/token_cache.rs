use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCache {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Unix timestamp (seconds) when access_token expires.
    pub expires_at: Option<u64>,
    pub client_id: String,
    /// The Zitadel host this token belongs to.
    pub host: String,
}

impl TokenCache {
    pub fn load() -> Result<Option<Self>> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(None);
        }
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("failed to read token cache {}", path.display()))?;
        let cache =
            serde_json::from_str(&contents).with_context(|| "failed to parse token cache")?;
        Ok(Some(cache))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_string_pretty(self)?;
        write_secure_file(&path, &contents)
    }

    pub fn clear() -> Result<()> {
        let path = Self::path()?;
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove {}", path.display()))?;
        }
        Ok(())
    }

    pub fn is_expired(&self) -> bool {
        let Some(expires_at) = self.expires_at else {
            return true;
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now >= expires_at
    }

    /// Returns the token cache path.
    /// Respects `ZITADEL_TUI_TOKEN_CACHE` env var to allow tests to redirect
    /// to a temp path without touching the real user cache.
    pub fn path() -> Result<PathBuf> {
        if let Ok(p) = std::env::var("ZITADEL_TUI_TOKEN_CACHE") {
            return Ok(PathBuf::from(p));
        }
        token_cache_default_path()
    }
}

#[cfg(test)]
fn token_cache_default_path() -> Result<PathBuf> {
    Ok(std::env::temp_dir().join(format!(
        "zitadel-tui-test-default-tokens-{}.json",
        std::process::id()
    )))
}

#[cfg(not(test))]
fn token_cache_default_path() -> Result<PathBuf> {
    dirs::config_dir()
        .map(|d| d.join("zitadel-tui").join("tokens.json"))
        .context("could not determine config directory")
}
#[cfg(unix)]
fn write_secure_file(path: &std::path::Path, contents: &str) -> Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}

#[cfg(not(unix))]
fn write_secure_file(path: &std::path::Path, contents: &str) -> Result<()> {
    fs::write(path, contents)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cache(expires_offset_secs: i64) -> TokenCache {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        TokenCache {
            access_token: "acc".to_string(),
            refresh_token: Some("ref".to_string()),
            expires_at: Some((now as i64 + expires_offset_secs) as u64),
            client_id: "client-1".to_string(),
            host: "https://zitadel.example.com".to_string(),
        }
    }

    #[test]
    fn expired_token_detected() {
        assert!(make_cache(-10).is_expired());
    }

    #[test]
    fn valid_token_not_expired() {
        assert!(!make_cache(300).is_expired());
    }

    #[test]
    fn no_expiry_treated_as_expired() {
        let cache = TokenCache {
            access_token: "acc".to_string(),
            refresh_token: None,
            expires_at: None,
            client_id: "client-1".to_string(),
            host: "https://zitadel.example.com".to_string(),
        };
        assert!(cache.is_expired());
    }

    #[test]
    fn round_trips_through_json() {
        let cache = make_cache(3600);
        let json = serde_json::to_string(&cache).unwrap();
        let back: TokenCache = serde_json::from_str(&json).unwrap();
        assert_eq!(back.access_token, "acc");
        assert_eq!(back.host, "https://zitadel.example.com");
    }
}
