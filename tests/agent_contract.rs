//! Agent-facing contract tests: the headless CLI must always emit a JSON
//! envelope on stdout, carry typed `error.kind` values, and exit with real
//! status codes (0 success, 1 command error, 2 clap parse error).

use std::{
    env, fs,
    path::PathBuf,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use mockito::Server;
use serde_json::Value;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_zitadel-tui"))
}

/// An empty TOML config so the real user config can never leak into a test.
fn empty_config() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = env::temp_dir().join(format!("zitadel-tui-contract-config-{unique}.toml"));
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
