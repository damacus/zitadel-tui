# OIDC Device Flow Login/Logout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `auth login` (Device Authorization Grant) and `auth logout` to zitadel-tui, storing tokens in a separate cache file with auto-refresh.

**Architecture:** A new `token_cache.rs` manages the `~/.config/zitadel-tui/tokens.json` file (0600 perms). A new `oidc.rs` implements the three Zitadel Device Flow HTTP calls (authorize, poll, refresh). `auth.rs` gains a token-cache source at the end of its resolution chain. `cli.rs` and `main.rs` wire up the two new subcommands.

**Tech Stack:** Rust, reqwest (already in deps), serde_json, dirs, tokio, rpassword (for prompting client_id, already in deps).

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `src/token_cache.rs` | **Create** | Load/save/clear `~/.config/zitadel-tui/tokens.json`; expiry check |
| `src/oidc.rs` | **Create** | `device_authorize`, `poll_for_token`, `refresh_access_token` |
| `src/cli.rs` | **Modify** | Add `Login(LoginArgs)` and `Logout` variants to `AuthAction`; add `LoginArgs` |
| `src/config.rs` | **Modify** | Add `device_client_id: Option<String>` to `AppConfig` |
| `src/auth.rs` | **Modify** | Add token-cache source (with auto-refresh) at end of `resolve_access_token` |
| `src/main.rs` | **Modify** | Handle `AuthAction::Login` and `AuthAction::Logout` in `execute_auth_command` |
| `src/conductor.rs` | **Modify** | Recognise "session token" as valid auth in `setup_required` / `auth_label` |

---

## Task 1: Token cache file (`src/token_cache.rs`)

**Files:**
- Create: `src/token_cache.rs`
- Modify: `src/main.rs` (add `mod token_cache;`)

- [ ] **Step 1: Write failing tests**

Add to the bottom of `src/token_cache.rs` (create the file with tests only first):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn make_cache(expires_offset_secs: i64) -> TokenCache {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
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
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cargo test token_cache 2>&1
```
Expected: compile error (module doesn't exist yet).

- [ ] **Step 3: Implement `src/token_cache.rs`**

```rust
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
        let cache = serde_json::from_str(&contents)
            .with_context(|| "failed to parse token cache")?;
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
        dirs::config_dir()
            .map(|d| d.join("zitadel-tui").join("tokens.json"))
            .context("could not determine config directory")
    }
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

// --- tests (from Step 1) ---
```

- [ ] **Step 4: Add `mod token_cache;` to `src/main.rs`** (top of file, with the other mods)

- [ ] **Step 5: Run tests**

```bash
cargo test token_cache 2>&1
```
Expected: 4 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/token_cache.rs src/main.rs
git commit -m "feat: add token cache for OIDC session storage"
```

---

## Task 2: OIDC device flow (`src/oidc.rs`)

Zitadel Device Flow endpoints:
- **Authorize:** `POST {host}/oauth/v2/device_authorization`
  Body (form): `client_id`, `scope`
- **Poll:** `POST {host}/oauth/v2/token`
  Body (form): `grant_type=urn:ietf:params:oauth:grant-type:device_code`, `client_id`, `device_code`
- **Refresh:** `POST {host}/oauth/v2/token`
  Body (form): `grant_type=refresh_token`, `client_id`, `refresh_token`

**Files:**
- Create: `src/oidc.rs`
- Modify: `src/main.rs` (add `mod oidc;`)

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn device_authorize_returns_codes() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/oauth/v2/device_authorization")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "device_code": "dev-code-1",
                "user_code": "ABCD-1234",
                "verification_uri": "https://example.com/activate",
                "verification_uri_complete": "https://example.com/activate?user_code=ABCD-1234",
                "expires_in": 300,
                "interval": 5
            }"#)
            .create_async()
            .await;

        let http = reqwest::Client::new();
        let resp = device_authorize(&http, &server.url(), "client-1").await.unwrap();
        assert_eq!(resp.device_code, "dev-code-1");
        assert_eq!(resp.user_code, "ABCD-1234");
        assert_eq!(resp.interval, 5);
    }

    #[tokio::test]
    async fn poll_returns_tokens_on_success() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/oauth/v2/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "access_token": "acc-tok",
                "refresh_token": "ref-tok",
                "expires_in": 3600
            }"#)
            .create_async()
            .await;

        let http = reqwest::Client::new();
        let tokens = poll_for_token(&http, &server.url(), "client-1", "dev-code-1")
            .await
            .unwrap();
        assert_eq!(tokens.access_token, "acc-tok");
        assert_eq!(tokens.refresh_token.as_deref(), Some("ref-tok"));
        assert_eq!(tokens.expires_in, 3600);
    }

    #[tokio::test]
    async fn refresh_returns_new_access_token() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/oauth/v2/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"access_token": "new-acc", "expires_in": 3600}"#)
            .create_async()
            .await;

        let http = reqwest::Client::new();
        let tokens = refresh_access_token(&http, &server.url(), "client-1", "ref-tok")
            .await
            .unwrap();
        assert_eq!(tokens.access_token, "new-acc");
    }

    #[tokio::test]
    async fn poll_returns_pending_error_on_authorization_pending() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/oauth/v2/token")
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "authorization_pending"}"#)
            .create_async()
            .await;

        let http = reqwest::Client::new();
        let err = poll_for_token(&http, &server.url(), "client-1", "dev-code-1")
            .await
            .unwrap_err();
        assert!(matches!(err, PollError::Pending));
    }
}
```

- [ ] **Step 2: Run tests to confirm compile error**

```bash
cargo test oidc 2>&1
```

- [ ] **Step 3: Implement `src/oidc.rs`**

```rust
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DeviceAuthResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    #[serde(default)]
    pub verification_uri_complete: Option<String>,
    pub expires_in: u64,
    /// Minimum polling interval in seconds.
    #[serde(default = "default_interval")]
    pub interval: u64,
}

fn default_interval() -> u64 { 5 }

#[derive(Debug)]
pub struct OidcTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
}

/// Returned by poll_for_token when not yet approved.
#[derive(Debug)]
pub enum PollError {
    /// User hasn't approved yet — keep polling.
    Pending,
    /// Slow down — increase interval.
    SlowDown,
    /// Fatal error.
    Fatal(anyhow::Error),
}

impl From<anyhow::Error> for PollError {
    fn from(e: anyhow::Error) -> Self { PollError::Fatal(e) }
}

const SCOPES: &str =
    "openid profile email offline_access urn:zitadel:iam:org:project:id:zitadel:aud";

pub async fn device_authorize(
    client: &Client,
    host: &str,
    client_id: &str,
) -> Result<DeviceAuthResponse> {
    let url = format!("{}/oauth/v2/device_authorization", host.trim_end_matches('/'));
    let params = [("client_id", client_id), ("scope", SCOPES)];
    let resp = client.post(&url).form(&params).send().await?;
    let status = resp.status();
    let body = resp.bytes().await?;
    if !status.is_success() {
        anyhow::bail!("device authorization request failed ({status})");
    }
    serde_json::from_slice(&body).context("failed to parse device authorization response")
}

pub async fn poll_for_token(
    client: &Client,
    host: &str,
    client_id: &str,
    device_code: &str,
) -> std::result::Result<OidcTokens, PollError> {
    let url = format!("{}/oauth/v2/token", host.trim_end_matches('/'));
    let params = [
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ("client_id", client_id),
        ("device_code", device_code),
    ];
    let resp = client.post(&url).form(&params).send().await
        .context("poll request failed")?;
    let status = resp.status();
    let body = resp.bytes().await.context("failed to read poll response")?;

    if status.is_success() {
        return parse_tokens(&body).map_err(PollError::Fatal);
    }

    let json: serde_json::Value = serde_json::from_slice(&body)
        .unwrap_or_default();
    match json.get("error").and_then(|e| e.as_str()) {
        Some("authorization_pending") => Err(PollError::Pending),
        Some("slow_down") => Err(PollError::SlowDown),
        Some(other) => Err(PollError::Fatal(anyhow!("device poll error: {other}"))),
        None => Err(PollError::Fatal(anyhow!("device poll failed ({status})"))),
    }
}

pub async fn refresh_access_token(
    client: &Client,
    host: &str,
    client_id: &str,
    refresh_token: &str,
) -> Result<OidcTokens> {
    let url = format!("{}/oauth/v2/token", host.trim_end_matches('/'));
    let params = [
        ("grant_type", "refresh_token"),
        ("client_id", client_id),
        ("refresh_token", refresh_token),
    ];
    let resp = client.post(&url).form(&params).send().await?;
    let status = resp.status();
    let body = resp.bytes().await?;
    if !status.is_success() {
        anyhow::bail!("token refresh failed ({status})");
    }
    parse_tokens(&body)
}

fn parse_tokens(body: &[u8]) -> Result<OidcTokens> {
    let json: serde_json::Value =
        serde_json::from_slice(body).context("failed to parse token response")?;
    let access_token = json
        .get("access_token")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("token response missing access_token"))?;
    Ok(OidcTokens {
        access_token,
        refresh_token: json
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned),
        expires_in: json
            .get("expires_in")
            .and_then(|v| v.as_u64())
            .unwrap_or(3600),
    })
}

/// Compute the unix timestamp when a token will expire,
/// with a 30-second safety margin.
pub fn expires_at_from_now(expires_in: u64) -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    now + expires_in.saturating_sub(30)
}

// --- tests (from Step 1) ---
```

- [ ] **Step 4: Add `mod oidc;` to `src/main.rs`**

- [ ] **Step 5: Run tests**

```bash
cargo test oidc 2>&1
```
Expected: 4 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/oidc.rs src/main.rs
git commit -m "feat: implement OIDC device authorization flow"
```

---

## Task 3: Update CLI (`src/cli.rs`)

**Files:**
- Modify: `src/cli.rs`

- [ ] **Step 1: Write failing tests first**

Add these three tests to the `#[cfg(test)] mod tests` block in `src/cli.rs` (before touching `AuthAction`):

```rust
#[test]
fn parses_auth_login() {
    let cli = Cli::parse_from(["zitadel-tui", "--once", "auth", "login"]);
    assert!(matches!(
        cli.command,
        Some(Command::Auth(AuthCommand {
            action: AuthAction::Login(LoginArgs { client_id: None })
        }))
    ));
}

#[test]
fn parses_auth_login_with_client_id() {
    let cli = Cli::parse_from([
        "zitadel-tui", "--once", "auth", "login", "--client-id", "my-client",
    ]);
    match cli.command {
        Some(Command::Auth(AuthCommand {
            action: AuthAction::Login(args),
        })) => assert_eq!(args.client_id.as_deref(), Some("my-client")),
        _ => panic!("expected auth login"),
    }
}

#[test]
fn parses_auth_logout() {
    let cli = Cli::parse_from(["zitadel-tui", "--once", "auth", "logout"]);
    assert!(matches!(
        cli.command,
        Some(Command::Auth(AuthCommand {
            action: AuthAction::Logout
        }))
    ));
}
```

- [ ] **Step 2: Run tests to confirm compile failure**

```bash
cargo test cli::tests 2>&1
```
Expected: compile error — `AuthAction::Login`, `LoginArgs`, and `AuthAction::Logout` do not exist.

- [ ] **Step 3: Add `Login(LoginArgs)` and `Logout` to `AuthAction`**

In `src/cli.rs`, change:
```rust
#[derive(Subcommand, Debug, Clone)]
pub enum AuthAction {
    Validate,
}
```
to:
```rust
#[derive(Subcommand, Debug, Clone)]
pub enum AuthAction {
    Login(LoginArgs),
    Logout,
    Validate,
}

#[derive(clap::Args, Debug, Clone)]
pub struct LoginArgs {
    #[arg(long)]
    pub client_id: Option<String>,
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test cli::tests 2>&1
```
Expected: all pass (including 3 new tests).

- [ ] **Step 5: Commit**

```bash
git add src/cli.rs
git commit -m "feat: add auth login and logout CLI subcommands"
```

---

## Task 4: Add `device_client_id` to config (`src/config.rs`)

**Files:**
- Modify: `src/config.rs`

- [ ] **Step 1: Write failing test first**

Add to `config.rs` tests:
```rust
#[test]
fn load_device_client_id_from_toml() {
    let config = AppConfig::load_from_str(
        r#"device_client_id = "native-app-client-id""#,
    ).unwrap();
    assert_eq!(config.device_client_id.as_deref(), Some("native-app-client-id"));
}
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cargo test config::tests::load_device_client_id_from_toml 2>&1
```
Expected: compile error — field does not exist on `AppConfig`.

- [ ] **Step 3: Add field to `AppConfig` and `PersistedAppConfig`**

In `AppConfig` (add after `service_account_file`):
```rust
pub device_client_id: Option<String>,
```

Also add the same field to `PersistedAppConfig`. Then:

- Update `From<&AppConfig> for PersistedAppConfig`: add `device_client_id: value.device_client_id.clone()`
- Update the manual `Debug` impl: add `.field("device_client_id", &self.device_client_id)`
- Update the manual `Serialize` impl: change `serialize_struct("AppConfig", 5)` to `6`, and add:
  `state.serialize_field("device_client_id", &self.device_client_id)?;`

**IMPORTANT — fix existing struct literals:** The tests in `config.rs` at `serialization_redacts_tokens` and `save_writes_secret_config_with_restricted_permissions` construct `AppConfig { ... }` with all fields named explicitly. Add `device_client_id: None` to each, or change them to use `..Default::default()`. For example:
```rust
let config = AppConfig {
    zitadel_url: Some("https://zitadel.example.com".to_string()),
    project_id: Some("123".to_string()),
    apps_config_file: Some(PathBuf::from("/tmp/apps.yml")),
    pat: Some("secret-token".to_string()),
    service_account_file: Some(PathBuf::from("/tmp/sa.json")),
    device_client_id: None,  // ADD THIS
};
```
Similarly update any `AppConfig { ... }` struct literals in `auth.rs` tests (they use `AppConfig::default()` already — no change needed there).

- [ ] **Step 4: Run tests**

```bash
cargo test config::tests 2>&1
```
Expected: all pass including the new test.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat: add device_client_id to AppConfig"
```

---

## Task 5: Wire token cache into auth resolution (`src/auth.rs`)

**Files:**
- Modify: `src/auth.rs`

- [ ] **Step 1: Write failing tests first** (see Step 2 below for the test code)

Add the `temp_cache_path` helper and the two new tests to the `auth.rs` test block. Run:

```bash
cargo test auth::tests::resolves_from_valid_token_cache 2>&1
```
Expected: compile error — `crate::token_cache` does not exist yet (if Task 1 is not yet done) or the test runs and fails because `resolve_access_token` does not yet check the cache.

- [ ] **Step 2: Implement — add token cache as final source in `resolve_access_token`**

At the end of `resolve_access_token`, before the `bail!`, add:

```rust
// Token cache (OIDC session) — with auto-refresh
if let Ok(Some(cache)) = crate::token_cache::TokenCache::load() {
    if !cache.is_expired() {
        return Ok(ResolvedAuth {
            token: cache.access_token,
            source: "session token",
        });
    }
    // Try to refresh
    if let Some(refresh_token) = &cache.refresh_token {
        let host = &cache.host;
        match crate::oidc::refresh_access_token(client, host, &cache.client_id, refresh_token).await {
            Ok(tokens) => {
                let updated = crate::token_cache::TokenCache {
                    access_token: tokens.access_token.clone(),
                    refresh_token: tokens.refresh_token.or_else(|| Some(refresh_token.clone())),
                    expires_at: Some(crate::oidc::expires_at_from_now(tokens.expires_in)),
                    client_id: cache.client_id,
                    host: cache.host,
                };
                let _ = updated.save(); // best-effort
                return Ok(ResolvedAuth {
                    token: tokens.access_token,
                    source: "session token (refreshed)",
                });
            }
            Err(_) => {} // fall through to bail
        }
    }
}
```

- [ ] **Step 3: Test code (added in Step 1)**

These tests use `ZITADEL_TUI_TOKEN_CACHE` to redirect to a temp file so they never touch the real user cache. They need `use mockito::Server;` which is already imported in the existing auth test block.

Add a `temp_cache_path` helper at the top of the `auth.rs` test module (near `temp_file`):
```rust
fn temp_cache_path() -> std::path::PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    env::temp_dir().join(format!("zitadel-tui-test-tokens-{unique}.json"))
}
```

Then add the two new tests:

```rust
#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn resolves_from_valid_token_cache() {
    let _guard = test_lock();
    let cache_path = temp_cache_path();
    env::set_var("ZITADEL_TUI_TOKEN_CACHE", &cache_path);
    let original_token = env::var("ZITADEL_TOKEN").ok();
    let original_sa = env::var("ZITADEL_SERVICE_ACCOUNT_FILE").ok();
    env::remove_var("ZITADEL_TOKEN");
    env::remove_var("ZITADEL_SERVICE_ACCOUNT_FILE");

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let cache = crate::token_cache::TokenCache {
        access_token: "cached-token".to_string(),
        refresh_token: None,
        expires_at: Some(now + 3600),
        client_id: "c1".to_string(),
        host: "https://zitadel.example.com".to_string(),
    };
    cache.save().unwrap();

    let http = Client::new();
    let config = AppConfig::default();
    let auth = resolve_access_token(&http, "https://zitadel.example.com", None, None, &config)
        .await.unwrap();

    env::remove_var("ZITADEL_TUI_TOKEN_CACHE");
    let _ = std::fs::remove_file(&cache_path);
    if let Some(v) = original_token { env::set_var("ZITADEL_TOKEN", v); }
    if let Some(v) = original_sa { env::set_var("ZITADEL_SERVICE_ACCOUNT_FILE", v); }

    assert_eq!(auth.token, "cached-token");
    assert_eq!(auth.source, "session token");
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn auto_refreshes_expired_token_cache() {
    let mut server = Server::new_async().await;
    server
        .mock("POST", "/oauth/v2/token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"access_token":"refreshed-token","expires_in":3600}"#)
        .create_async()
        .await;

    let _guard = test_lock();
    let cache_path = temp_cache_path();
    env::set_var("ZITADEL_TUI_TOKEN_CACHE", &cache_path);
    let original_token = env::var("ZITADEL_TOKEN").ok();
    let original_sa = env::var("ZITADEL_SERVICE_ACCOUNT_FILE").ok();
    env::remove_var("ZITADEL_TOKEN");
    env::remove_var("ZITADEL_SERVICE_ACCOUNT_FILE");

    let cache = crate::token_cache::TokenCache {
        access_token: "expired-token".to_string(),
        refresh_token: Some("my-refresh-token".to_string()),
        expires_at: Some(0), // expired
        client_id: "c1".to_string(),
        host: server.url(),
    };
    cache.save().unwrap();

    let http = Client::new();
    let config = AppConfig::default();
    let auth = resolve_access_token(&http, &server.url(), None, None, &config)
        .await.unwrap();

    env::remove_var("ZITADEL_TUI_TOKEN_CACHE");
    let _ = std::fs::remove_file(&cache_path);
    if let Some(v) = original_token { env::set_var("ZITADEL_TOKEN", v); }
    if let Some(v) = original_sa { env::set_var("ZITADEL_SERVICE_ACCOUNT_FILE", v); }

    assert_eq!(auth.token, "refreshed-token");
    assert!(auth.source.contains("refreshed"));
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test auth::tests 2>&1
```
Expected: all pass including 2 new tests.

- [ ] **Step 5: Commit**

```bash
git add src/auth.rs
git commit -m "feat: resolve credentials from OIDC token cache with auto-refresh"
```

---

## Task 6: Wire `auth login` and `auth logout` into `main.rs`

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add a `prompt_for_client_id` helper**

```rust
fn prompt_for_client_id() -> Result<String> {
    eprint!("Zitadel native app client ID: ");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let id = input.trim().to_string();
    if id.is_empty() {
        anyhow::bail!("client ID cannot be empty");
    }
    Ok(id)
}
```

- [ ] **Step 2: Add login and logout handlers to `execute_auth_command`**

Replace the existing `execute_auth_command` body with:

```rust
async fn execute_auth_command(
    command: &AuthCommand,
    args: &Cli,
    config: &AppConfig,
) -> Result<Value> {
    match &command.action {
        AuthAction::Login(login_args) => {
            let host = resolved_host(args, config)?;

            // Resolve client_id: flag > config > prompt (then save)
            let client_id = if let Some(id) = &login_args.client_id {
                id.clone()
            } else if let Some(id) = &config.device_client_id {
                id.clone()
            } else {
                let id = prompt_for_client_id()?;
                // Save to config for future use
                let mut updated = config.clone();
                updated.device_client_id = Some(id.clone());
                let _ = updated.save_to_canonical_path();
                id
            };

            let http = reqwest::Client::new();
            let auth_resp = oidc::device_authorize(&http, &host, &client_id).await?;

            eprintln!("\nOpen this URL in your browser:");
            eprintln!("  {}", auth_resp.verification_uri_complete.as_deref().unwrap_or(&auth_resp.verification_uri));
            eprintln!("\nOr go to {} and enter code: {}\n", auth_resp.verification_uri, auth_resp.user_code);

            // Poll until approved, denied, or expired
            let mut interval = auth_resp.interval;
            let tokens = loop {
                tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                match oidc::poll_for_token(&http, &host, &client_id, &auth_resp.device_code).await {
                    Ok(tokens) => break tokens,
                    Err(oidc::PollError::Pending) => {
                        eprint!(".");
                    }
                    Err(oidc::PollError::SlowDown) => {
                        interval += 5;
                        eprint!(".");
                    }
                    Err(oidc::PollError::Fatal(e)) => return Err(e),
                }
            };
            eprintln!("\nAuthenticated.");

            let cache = token_cache::TokenCache {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                expires_at: Some(oidc::expires_at_from_now(tokens.expires_in)),
                client_id,
                host: host.clone(),
            };
            cache.save()?;

            Ok(serde_json::json!({
                "status": "authenticated",
                "host": host,
                "token_cache": token_cache::TokenCache::path()?.display().to_string(),
            }))
        }
        AuthAction::Logout => {
            token_cache::TokenCache::clear()?;
            Ok(serde_json::json!({ "status": "logged out" }))
        }
        AuthAction::Validate => {
            let host = resolved_host(args, config)?;
            let http = reqwest::Client::new();
            let auth = resolve_access_token(
                &http,
                &host,
                args.token.clone(),
                args.service_account_file.clone(),
                config,
            )
            .await?;
            let client = ZitadelClient::new(host.clone(), auth.token)?;
            let projects = client.list_projects().await?;
            Ok(serde_json::json!({
                "host": host,
                "auth_source": auth.source,
                "project_count": projects.len(),
            }))
        }
    }
}
```

- [ ] **Step 3: Update `use` imports at top of `main.rs`**

`token_cache` and `oidc` are already declared as modules (added in Tasks 1 and 2). The `cli::` import line does not need `LoginArgs` — the `Login(login_args)` pattern match works without importing the type explicitly. The existing import line is unchanged; only the new `AuthAction::Login` and `AuthAction::Logout` match arms in `execute_auth_command` are enough.

- [ ] **Step 4: Add `time` feature to the existing `tokio` line in `Cargo.toml`**

Find the existing line:
```toml
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```
**Replace it** (do not add a second `tokio` entry) with:
```toml
tokio = { version = "1", features = ["macros", "rt-multi-thread", "time"] }
```

- [ ] **Step 5: Build and check**

```bash
cargo build 2>&1
```
Expected: compiles with no errors.

- [ ] **Step 6: Commit**

```bash
git add src/main.rs Cargo.toml
git commit -m "feat: implement auth login (device flow) and auth logout commands"
```

---

## Task 7: Update `conductor.rs` auth label

**Files:**
- Modify: `src/conductor.rs`

- [ ] **Step 1: Recognise session token in `setup_required` check**

In `refresh_runtime`, the current check is:
```rust
self.setup_required = self.config.zitadel_url.is_none()
    || (self.config.pat.is_none() && self.config.service_account_file.is_none());
```

Replace with:
```rust
let has_credential = self.config.pat.is_some()
    || self.config.service_account_file.is_some()
    || self.cli.token.is_some()
    || crate::token_cache::TokenCache::load()
        .ok()
        .flatten()
        .map(|c| !c.is_expired())
        .unwrap_or(false);
self.setup_required = self.config.zitadel_url.is_none() || !has_credential;
```

- [ ] **Step 2: Run full test suite**

```bash
cargo test 2>&1
```
Expected: all tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/conductor.rs
git commit -m "feat: recognise OIDC session token in TUI setup check"
```

---

## Task 8: Manual end-to-end test

- [ ] **Build release**
```bash
cargo build --release
```

- [ ] **Test logout (idempotent when no cache exists)**
```bash
./target/release/zitadel-tui --once auth logout
```
Expected: `{ "status": "logged out" }`

- [ ] **Test login (requires a native app with device flow enabled in your Zitadel instance)**
```bash
./target/release/zitadel-tui --once --host https://zitadel.damacus.io auth login --client-id <your-native-client-id>
```
Expected: URL + user code printed, approval via browser, `"status": "authenticated"` JSON.

- [ ] **Test that subsequent commands use the cached token**
```bash
./target/release/zitadel-tui --once --host https://zitadel.damacus.io apps list
```
Expected: apps listed without needing `--token`.

- [ ] **Test validate shows session token source**
```bash
./target/release/zitadel-tui --once --host https://zitadel.damacus.io auth status
```
Expected: `"auth_source": "session token"`

- [ ] **Test logout clears the session**
```bash
./target/release/zitadel-tui --once auth logout
./target/release/zitadel-tui --once --host https://zitadel.damacus.io auth status
```
Expected: second command fails with "no credentials available".
