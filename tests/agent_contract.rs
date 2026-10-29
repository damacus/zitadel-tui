//! Agent-facing contract tests: the headless CLI must always emit a JSON
//! envelope on stdout, carry typed `error.kind` values, and exit with real
//! status codes (0 success, 1 command error, 2 clap parse error).

use std::{
    env, fs,
    path::PathBuf,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use mockito::{Matcher, Server};
use serde_json::{json, Value};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_zitadel-tui"))
}

/// An empty TOML config so the real user config can never leak into a test.
/// Unique per call (pid + nanos + counter) — parallel tests must never
/// remove each other's file.
fn empty_config() -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let seq = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let path = env::temp_dir().join(format!(
        "zitadel-tui-contract-config-{}-{unique}-{seq}.toml",
        std::process::id()
    ));
    fs::write(&path, "").unwrap();
    path
}

fn stdout_json(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout is not valid JSON: {error}\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn apps_list_prints_success_envelope_without_json_flag() {
    let mut server = Server::new();
    server
        .mock("POST", "/management/v1/projects/proj-1/apps/_search")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"result":[{"id":"app-1","name":"grafana"}]}"#)
        .create();

    let config = empty_config();
    let output = bin()
        .args([
            "--host",
            &server.url(),
            "--token",
            "test",
            "--project-id",
            "proj-1",
            "--config",
        ])
        .arg(&config)
        .args(["apps", "list"])
        .output()
        .expect("failed to spawn zitadel-tui");

    let _ = fs::remove_file(&config);
    assert_eq!(output.status.code(), Some(0));
    let envelope = stdout_json(&output);
    assert_eq!(envelope["command"], "apps.list");
    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["result"][0]["id"], "app-1");
    assert_eq!(envelope["result"][0]["name"], "grafana");
}

#[test]
fn api_failure_prints_error_envelope_and_exits_1() {
    let mut server = Server::new();
    server
        .mock("POST", "/management/v1/projects/proj-1/apps/_search")
        .with_status(500)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":"boom"}"#)
        .create();

    let config = empty_config();
    let output = bin()
        .args([
            "--host",
            &server.url(),
            "--token",
            "test",
            "--project-id",
            "proj-1",
            "--config",
        ])
        .arg(&config)
        .args(["apps", "list"])
        .output()
        .expect("failed to spawn zitadel-tui");

    let _ = fs::remove_file(&config);
    assert_eq!(output.status.code(), Some(1));
    let envelope = stdout_json(&output);
    assert_eq!(envelope["command"], "apps.list");
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["error"]["kind"], "api");
    assert!(envelope["error"]["message"].is_string());
    // API error bodies must not leak into the message.
    assert!(!envelope["error"]["message"]
        .as_str()
        .unwrap()
        .contains("boom"));
}

#[test]
fn apps_delete_unresolved_id_surfaces_api_kind() {
    // `apps delete` does not resolve by name today; a missing app hits the
    // DELETE endpoint directly, so the pinned kind is `api`.
    let mut server = Server::new();
    server
        .mock("DELETE", "/management/v1/projects/proj-1/apps/missing")
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":"not found"}"#)
        .create();

    let config = empty_config();
    let output = bin()
        .args([
            "--host",
            &server.url(),
            "--token",
            "test",
            "--project-id",
            "proj-1",
            "--config",
        ])
        .arg(&config)
        .args(["apps", "delete", "--app-id", "missing"])
        .output()
        .expect("failed to spawn zitadel-tui");

    let _ = fs::remove_file(&config);
    assert_eq!(output.status.code(), Some(1));
    let envelope = stdout_json(&output);
    assert_eq!(envelope["command"], "apps.delete");
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["error"]["kind"], "api");
}

#[test]
fn auth_login_without_client_id_is_usage_error_and_does_not_hang() {
    let config = empty_config();
    let output = bin()
        .args(["--host", "https://zitadel.example.com", "--config"])
        .arg(&config)
        .args(["auth", "login"])
        .stdin(Stdio::null())
        .output()
        .expect("failed to spawn zitadel-tui");

    let _ = fs::remove_file(&config);
    assert_eq!(output.status.code(), Some(1));
    let envelope = stdout_json(&output);
    assert_eq!(envelope["command"], "auth.login");
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["error"]["kind"], "usage");
    assert_eq!(
        envelope["error"]["message"],
        "client ID required: pass --client-id or set device_client_id in config"
    );
}

#[test]
fn users_create_admin_without_password_is_usage_not_auth() {
    // No credentials anywhere: with validation ordered before auth
    // resolution this must still surface `usage`, never `auth`.
    let config = empty_config();
    let output = bin()
        .env_remove("ZITADEL_URL")
        .env_remove("ZITADEL_TOKEN")
        .env_remove("ZITADEL_SERVICE_ACCOUNT_FILE")
        .env_remove("ZITADEL_PROJECT_ID")
        .env_remove("ZITADEL_TUI_TOKEN_CACHE")
        .args(["--host", "https://example.invalid", "--config"])
        .arg(&config)
        .args([
            "users",
            "create-admin",
            "--username",
            "admin",
            "--first-name",
            "A",
            "--last-name",
            "B",
            "--email",
            "admin@example.com",
        ])
        .stdin(Stdio::null())
        .output()
        .expect("failed to spawn zitadel-tui");

    let _ = fs::remove_file(&config);
    assert_eq!(output.status.code(), Some(1));
    let envelope = stdout_json(&output);
    assert_eq!(envelope["command"], "users.create-admin");
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["error"]["kind"], "usage");
    assert_eq!(
        envelope["error"]["message"],
        "--password is required for users create-admin in headless mode"
    );
}

#[test]
fn idps_configure_google_without_client_secret_is_usage_not_auth() {
    let config = empty_config();
    let output = bin()
        .env_remove("ZITADEL_URL")
        .env_remove("ZITADEL_TOKEN")
        .env_remove("ZITADEL_SERVICE_ACCOUNT_FILE")
        .env_remove("ZITADEL_PROJECT_ID")
        .env_remove("ZITADEL_TUI_TOKEN_CACHE")
        .args(["--host", "https://example.invalid", "--config"])
        .arg(&config)
        .args(["idps", "configure-google", "--client-id", "google-cid"])
        .stdin(Stdio::null())
        .output()
        .expect("failed to spawn zitadel-tui");

    let _ = fs::remove_file(&config);
    assert_eq!(output.status.code(), Some(1));
    let envelope = stdout_json(&output);
    assert_eq!(envelope["command"], "idps.configure-google");
    assert_eq!(envelope["error"]["kind"], "usage");
}

#[test]
fn apps_create_with_unknown_template_is_not_found_without_server() {
    // No credentials and no server needed: the template lookup is local,
    // so `not_found` must win over `auth`/`api`.
    let config = empty_config();
    let output = bin()
        .env_remove("ZITADEL_URL")
        .env_remove("ZITADEL_TOKEN")
        .env_remove("ZITADEL_SERVICE_ACCOUNT_FILE")
        .env_remove("ZITADEL_PROJECT_ID")
        .env_remove("ZITADEL_TUI_TOKEN_CACHE")
        .args(["--host", "https://example.invalid", "--config"])
        .arg(&config)
        .args(["apps", "create", "--template", "bogus"])
        .stdin(Stdio::null())
        .output()
        .expect("failed to spawn zitadel-tui");

    let _ = fs::remove_file(&config);
    assert_eq!(output.status.code(), Some(1));
    let envelope = stdout_json(&output);
    assert_eq!(envelope["command"], "apps.create");
    assert_eq!(envelope["error"]["kind"], "not_found");
    assert_eq!(envelope["error"]["message"], "template bogus not found");
    assert_eq!(envelope["error"]["details"]["lookup"]["value"], "bogus");
}

#[test]
fn apps_create_without_name_or_redirect_uris_is_usage_not_auth() {
    let config = empty_config();
    let output = bin()
        .env_remove("ZITADEL_URL")
        .env_remove("ZITADEL_TOKEN")
        .env_remove("ZITADEL_SERVICE_ACCOUNT_FILE")
        .env_remove("ZITADEL_PROJECT_ID")
        .env_remove("ZITADEL_TUI_TOKEN_CACHE")
        .args(["--host", "https://example.invalid", "--config"])
        .arg(&config)
        .args(["apps", "create"])
        .stdin(Stdio::null())
        .output()
        .expect("failed to spawn zitadel-tui");

    let _ = fs::remove_file(&config);
    assert_eq!(output.status.code(), Some(1));
    let envelope = stdout_json(&output);
    assert_eq!(envelope["command"], "apps.create");
    assert_eq!(envelope["error"]["kind"], "usage");
}

#[test]
fn unknown_flag_exits_2() {
    let status = bin()
        .arg("--definitely-not-a-flag")
        .status()
        .expect("failed to spawn zitadel-tui");

    assert_eq!(status.code(), Some(2));
}

#[test]
fn removed_json_flag_is_rejected() {
    let status = bin()
        .args(["--json", "apps", "list"])
        .status()
        .expect("failed to spawn zitadel-tui");

    assert_eq!(status.code(), Some(2));
}

#[test]
fn removed_once_flag_is_rejected() {
    let status = bin()
        .args(["--once", "apps", "list"])
        .status()
        .expect("failed to spawn zitadel-tui");

    assert_eq!(status.code(), Some(2));
}

/// Two-app fixture: an OIDC web app carrying fields the summary must drop
/// (`oidcConfig`, `creationDate`) plus an API app exercising the
/// api_config `client_id` fallback.
const APPS_LIST_BODY: &str = r#"{"result":[
    {"id":"app-1","name":"grafana","state":"APP_STATE_ACTIVE","creationDate":"2026-01-01T00:00:00Z","oidcConfig":{"clientId":"cid-1","authMethodType":"OIDC_AUTH_METHOD_TYPE_BASIC","appType":"OIDC_APP_TYPE_WEB","redirectUris":["https://grafana.example.com/cb"]}},
    {"id":"app-2","name":"Management API","state":"APP_STATE_ACTIVE","apiConfig":{"clientId":"cid-2","authMethodType":"API_AUTH_METHOD_TYPE_PRIVATE_KEY_JWT"}}
]}"#;

fn mock_apps_search(server: &mut Server) {
    server
        .mock("POST", "/management/v1/projects/proj-1/apps/_search")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(APPS_LIST_BODY)
        .create();
}

fn run_apps_list(server_url: &str, extra_args: &[&str]) -> std::process::Output {
    let config = empty_config();
    let output = bin()
        .args([
            "--host",
            server_url,
            "--token",
            "test",
            "--project-id",
            "proj-1",
            "--config",
        ])
        .arg(&config)
        .args(["apps", "list"])
        .args(extra_args)
        .output()
        .expect("failed to spawn zitadel-tui");
    let _ = fs::remove_file(&config);
    output
}

#[test]
fn apps_list_defaults_to_summary_projection() {
    let mut server = Server::new();
    mock_apps_search(&mut server);

    let output = run_apps_list(&server.url(), &[]);

    assert_eq!(output.status.code(), Some(0));
    let envelope = stdout_json(&output);
    assert_eq!(envelope["ok"], true);
    let items = envelope["result"].as_array().expect("result is an array");
    assert_eq!(items.len(), 2);

    // Projection keys only — no oidcConfig, creationDate, or raw extras.
    let keys: std::collections::BTreeSet<&str> = items[0]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        ["client_id", "id", "kind", "name", "redirect_uris", "state"]
            .into_iter()
            .collect()
    );
    assert_eq!(items[0]["id"], "app-1");
    assert_eq!(items[0]["name"], "grafana");
    assert_eq!(items[0]["client_id"], "cid-1");
    assert_eq!(items[0]["kind"], "confidential");
    assert_eq!(
        items[0]["redirect_uris"],
        serde_json::json!(["https://grafana.example.com/cb"])
    );
    assert_eq!(items[0]["state"], "APP_STATE_ACTIVE");

    // api_config fallback: client_id survives, redirect_uris defaults to [].
    assert_eq!(items[1]["client_id"], "cid-2");
    assert_eq!(items[1]["kind"], "api");
    assert_eq!(items[1]["redirect_uris"], serde_json::json!([]));
}

#[test]
fn apps_list_full_returns_raw_payloads() {
    let mut server = Server::new();
    mock_apps_search(&mut server);

    // Global flag after the subcommand — the position agents will use.
    let output = run_apps_list(&server.url(), &["--full"]);

    assert_eq!(output.status.code(), Some(0));
    let envelope = stdout_json(&output);
    let items = envelope["result"].as_array().expect("result is an array");
    assert_eq!(items.len(), 2);
    let raw: Vec<Value> = serde_json::from_str::<Value>(APPS_LIST_BODY).unwrap()["result"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(items[0], raw[0]);
    assert_eq!(items[1], raw[1]);
    assert!(items[0]["oidcConfig"].is_object());
}

const USERS_LIST_BODY: &str = r#"{"result":[
    {"id":"user-1","userName":"admin","state":"USER_STATE_ACTIVE","preferredLoginName":"admin@example.com","human":{"profile":{"displayName":"Admin User"},"email":{"email":"admin@example.com"}},"creationDate":"2026-01-01T00:00:00Z"},
    {"id":"user-2","userName":"service-account","state":"USER_STATE_ACTIVE","loginNames":["sa@example.com"],"machine":{"name":"cluster service account"}}
]}"#;

fn mock_users_search(server: &mut Server) {
    server
        .mock("POST", "/management/v1/users/_search")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(USERS_LIST_BODY)
        .create();
}

fn run_users_list(server_url: &str, extra_args: &[&str]) -> std::process::Output {
    let config = empty_config();
    let output = bin()
        .args(["--host", server_url, "--token", "test", "--config"])
        .arg(&config)
        .args(["users", "list"])
        .args(extra_args)
        .output()
        .expect("failed to spawn zitadel-tui");
    let _ = fs::remove_file(&config);
    output
}

#[test]
fn users_list_defaults_to_summary_projection() {
    let mut server = Server::new();
    mock_users_search(&mut server);

    let output = run_users_list(&server.url(), &[]);

    assert_eq!(output.status.code(), Some(0));
    let envelope = stdout_json(&output);
    let items = envelope["result"].as_array().expect("result is an array");
    assert_eq!(items.len(), 2);

    let keys: std::collections::BTreeSet<&str> = items[0]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        [
            "display_name",
            "email",
            "id",
            "login_name",
            "state",
            "user_type",
            "username"
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(items[0]["username"], "admin");
    assert_eq!(items[0]["login_name"], "admin@example.com");
    assert_eq!(items[0]["display_name"], "Admin User");
    assert_eq!(items[0]["email"], "admin@example.com");
    assert_eq!(items[0]["user_type"], "human");

    // Machine variant: user_type flips, login_name falls back to
    // loginNames[0], display_name falls back to the machine name.
    assert_eq!(items[1]["user_type"], "machine");
    assert_eq!(items[1]["login_name"], "sa@example.com");
    assert_eq!(items[1]["display_name"], "cluster service account");
    assert!(items[1]["email"].is_null());
}

#[test]
fn users_list_full_returns_raw_payloads() {
    let mut server = Server::new();
    mock_users_search(&mut server);

    let output = run_users_list(&server.url(), &["--full"]);

    assert_eq!(output.status.code(), Some(0));
    let envelope = stdout_json(&output);
    let items = envelope["result"].as_array().expect("result is an array");
    let raw: Vec<Value> = serde_json::from_str::<Value>(USERS_LIST_BODY).unwrap()["result"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(*items, raw);
    assert!(items[0]["human"].is_object());
}

const IDPS_LIST_BODY: &str = r#"{"result":[
    {"id":"idp-1","name":"Google","state":"IDP_STATE_ACTIVE","type":"IDP_TYPE_GOOGLE","creationDate":"2026-01-01T00:00:00Z"}
]}"#;

#[test]
fn idps_list_defaults_to_summary_projection() {
    let mut server = Server::new();
    server
        .mock("POST", "/admin/v1/idps/_search")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(IDPS_LIST_BODY)
        .create();

    let config = empty_config();
    let output = bin()
        .args(["--host", &server.url(), "--token", "test", "--config"])
        .arg(&config)
        .args(["idps", "list"])
        .output()
        .expect("failed to spawn zitadel-tui");
    let _ = fs::remove_file(&config);

    assert_eq!(output.status.code(), Some(0));
    let envelope = stdout_json(&output);
    let items = envelope["result"].as_array().expect("result is an array");
    assert_eq!(items.len(), 1);
    let keys: std::collections::BTreeSet<&str> = items[0]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, ["id", "name", "state", "type"].into_iter().collect());
    assert_eq!(items[0]["type"], "IDP_TYPE_GOOGLE");
    assert!(items[0].get("creationDate").is_none());
}

#[test]
fn idps_list_full_returns_raw_payload() {
    let mut server = Server::new();
    server
        .mock("POST", "/admin/v1/idps/_search")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(IDPS_LIST_BODY)
        .create();

    let config = empty_config();
    let output = bin()
        .args(["--host", &server.url(), "--token", "test", "--config"])
        .arg(&config)
        .args(["idps", "list", "--full"])
        .output()
        .expect("failed to spawn zitadel-tui");
    let _ = fs::remove_file(&config);

    assert_eq!(output.status.code(), Some(0));
    let envelope = stdout_json(&output);
    let raw: Vec<Value> = serde_json::from_str::<Value>(IDPS_LIST_BODY).unwrap()["result"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(envelope["result"].as_array().unwrap(), &raw);
}

const PROJECTS_LIST_BODY: &str = r#"{"result":[
    {"id":"proj-1","name":"Platform","state":"PROJECT_STATE_ACTIVE","creationDate":"2026-01-01T00:00:00Z"}
]}"#;

#[test]
fn projects_list_returns_summary_projection() {
    let mut server = Server::new();
    server
        .mock("POST", "/management/v1/projects/_search")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(PROJECTS_LIST_BODY)
        .create();

    let config = empty_config();
    let output = bin()
        .args(["--host", &server.url(), "--token", "test", "--config"])
        .arg(&config)
        .args(["projects", "list"])
        .output()
        .expect("failed to spawn zitadel-tui");
    let _ = fs::remove_file(&config);

    assert_eq!(output.status.code(), Some(0));
    let envelope = stdout_json(&output);
    assert_eq!(envelope["command"], "projects.list");
    assert_eq!(envelope["ok"], true);
    let items = envelope["result"].as_array().expect("result is an array");
    assert_eq!(items.len(), 1);
    let keys: std::collections::BTreeSet<&str> = items[0]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, ["id", "name", "state"].into_iter().collect());
    assert_eq!(items[0]["name"], "Platform");
    assert!(items[0].get("creationDate").is_none());
}

#[test]
fn apps_list_limit_reaches_search_body() {
    let mut server = Server::new();
    let search = server
        .mock("POST", "/management/v1/projects/proj-1/apps/_search")
        .match_body(Matcher::PartialJson(json!({ "query": { "limit": 1 } })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"result":[{"id":"app-1","name":"grafana"}]}"#)
        .create();

    let config = empty_config();
    let output = bin()
        .args([
            "--host",
            &server.url(),
            "--token",
            "test",
            "--project-id",
            "proj-1",
            "--config",
        ])
        .arg(&config)
        .args(["apps", "list", "--limit", "1"])
        .output()
        .expect("failed to spawn zitadel-tui");
    let _ = fs::remove_file(&config);

    search.assert();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout_json(&output)["command"], "apps.list");
}

#[test]
fn apps_list_limit_below_range_exits_2() {
    let status = bin()
        .args(["apps", "list", "--limit", "0"])
        .status()
        .expect("failed to spawn zitadel-tui");

    assert_eq!(status.code(), Some(2));
}
