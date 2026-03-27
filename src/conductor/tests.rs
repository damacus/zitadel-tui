use super::helpers::*;
use super::TuiConductor;
use crate::{
    cli::Cli,
    client::ZitadelClient,
    config::{AppConfig, AppTemplate, TemplatesFile, UserTemplate},
    tui::{CanvasMode, FormState, PendingAction, Record, ResourceKind},
};
use mockito::Matcher;

#[test]
fn split_csv_filters_empty_entries() {
    assert_eq!(
        split_csv("a, b, ,c"),
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );
}

#[test]
fn begin_config_unknown_action_browses() {
    let conductor = TuiConductor {
        cli: Cli {
            host: None,
            project_id: None,
            token: None,
            service_account_file: None,
            config: None,
            json: false,
            once: false,
            command: None,
        },
        config: AppConfig::default(),
        templates: TemplatesFile::default(),
        host: "https://zitadel.example.com".to_string(),
        project: "default".to_string(),
        auth_label: "PAT".to_string(),
        setup_required: false,
        client: None,
        app_records: vec![],
        user_records: vec![],
        idp_records: vec![],
    };

    let mode = conductor.begin_action(ResourceKind::Config, 1, None);
    assert!(matches!(mode, CanvasMode::Browse));
}

fn test_conductor(host: &str) -> TuiConductor {
    TuiConductor {
        cli: Cli {
            host: Some(host.to_string()),
            project_id: Some("project-1".to_string()),
            token: None,
            service_account_file: None,
            config: None,
            json: false,
            once: false,
            command: None,
        },
        config: AppConfig {
            zitadel_url: Some(host.to_string()),
            project_id: Some("project-1".to_string()),
            pat: Some("test-token".to_string()),
            ..AppConfig::default()
        },
        templates: TemplatesFile {
            apps: Default::default(),
            users: Default::default(),
        },
        host: host.to_string(),
        project: "project-1".to_string(),
        auth_label: "PAT".to_string(),
        setup_required: false,
        client: None,
        app_records: vec![],
        user_records: vec![],
        idp_records: vec![],
    }
}

fn app_template(name: &str) -> AppTemplate {
    AppTemplate {
        redirect_uris: vec![format!("https://{name}.example.com/callback")],
        public: false,
    }
}

#[tokio::test]
async fn refresh_runtime_loads_all_record_types() {
    let mut server = mockito::Server::new_async().await;
    let _auth = server
        .mock("POST", "/oauth/v2/token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"access_token":"test-token"}"#)
        .create_async()
        .await;
    let _projects = server
        .mock("POST", "/management/v1/projects/_search")
        .match_header("authorization", "Bearer test-token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"result":[{"id":"project-1"}]}"#)
        .create_async()
        .await;
    let _apps = server
        .mock("POST", "/management/v1/projects/project-1/apps/_search")
        .match_header("authorization", "Bearer test-token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"result":[{"id":"app-1","name":"grafana","oidcConfig":{"authMethodType":"OIDC_AUTH_METHOD_TYPE_BASIC","clientId":"cid-1","redirectUris":["https://grafana.example.com/callback"]},"state":"ACTIVE"}]}"#)
        .create_async()
        .await;
    let _users = server
        .mock("POST", "/management/v1/users/_search")
        .match_header("authorization", "Bearer test-token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"result":[{"id":"user-1","userName":"admin","state":"ACTIVE","human":{"email":{"email":"admin@example.com"},"profile":{"displayName":"Admin User"}}}]}"#)
        .create_async()
        .await;
    let _idps = server
        .mock("POST", "/admin/v1/idps/_search")
        .match_header("authorization", "Bearer test-token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"result":[{"id":"idp-1","name":"Google","state":"ACTIVE","type":"google"}]}"#,
        )
        .create_async()
        .await;

    let mut conductor = test_conductor(&server.url());
    conductor.refresh_runtime().await;

    assert_eq!(conductor.app_records.len(), 1);
    assert_eq!(conductor.user_records.len(), 1);
    assert_eq!(conductor.idp_records.len(), 1);
    assert_eq!(conductor.project, "project-1");
    assert_eq!(conductor.auth_label, "config PAT");
}

#[tokio::test]
async fn quick_setup_apps_batches_creates_and_refreshes() {
    let mut server = mockito::Server::new_async().await;
    let _auth = server
        .mock("POST", "/oauth/v2/token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"access_token":"test-token"}"#)
        .create_async()
        .await;
    let _projects = server
        .mock("POST", "/management/v1/projects/_search")
        .match_header("authorization", "Bearer test-token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"result":[{"id":"project-1"}]}"#)
        .create_async()
        .await;
    let create_grafana = server
        .mock("POST", "/management/v1/projects/project-1/apps/oidc")
        .match_header("authorization", "Bearer test-token")
        .match_body(Matcher::PartialJson(serde_json::json!({
            "name": "grafana",
            "redirectUris": ["https://grafana.example.com/callback"],
            "authMethodType": "OIDC_AUTH_METHOD_TYPE_BASIC"
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"clientId":"cid-grafana"}"#)
        .create_async()
        .await;
    let create_mealie = server
        .mock("POST", "/management/v1/projects/project-1/apps/oidc")
        .match_header("authorization", "Bearer test-token")
        .match_body(Matcher::PartialJson(serde_json::json!({
            "name": "mealie",
            "redirectUris": ["https://mealie.example.com/callback"],
            "authMethodType": "OIDC_AUTH_METHOD_TYPE_BASIC"
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"clientId":"cid-mealie"}"#)
        .create_async()
        .await;
    let _apps = server
        .mock("POST", "/management/v1/projects/project-1/apps/_search")
        .match_header("authorization", "Bearer test-token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"result":[]}"#)
        .create_async()
        .await;
    let _users = server
        .mock("POST", "/management/v1/users/_search")
        .match_header("authorization", "Bearer test-token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"result":[]}"#)
        .create_async()
        .await;
    let _idps = server
        .mock("POST", "/admin/v1/idps/_search")
        .match_header("authorization", "Bearer test-token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"result":[]}"#)
        .create_async()
        .await;

    let mut conductor = test_conductor(&server.url());
    conductor
        .templates
        .apps
        .insert("grafana".to_string(), app_template("grafana"));
    conductor
        .templates
        .apps
        .insert("mealie".to_string(), app_template("mealie"));
    conductor.client = Some(ZitadelClient::new(server.url(), "test-token".to_string()).unwrap());
    let form = FormState {
        title: "Quick setup applications".to_string(),
        description: String::new(),
        submit_label: String::new(),
        fields: vec![
            checkbox_field("template_app", "grafana", true, ""),
            checkbox_field("template_app", "mealie", true, ""),
        ],
        selected_field: 0,
        pending: PendingAction::QuickSetupApplications,
    };

    let mode = conductor.quick_setup_apps(&form).await;

    create_grafana.assert_async().await;
    create_mealie.assert_async().await;
    assert!(matches!(mode, CanvasMode::Success(_)));
    assert_eq!(conductor.app_records.len(), 0);
}

#[tokio::test]
async fn quick_setup_users_batches_creates_and_grants_admins() {
    let mut server = mockito::Server::new_async().await;
    let _auth = server
        .mock("POST", "/oauth/v2/token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"access_token":"test-token"}"#)
        .create_async()
        .await;
    let _projects = server
        .mock("POST", "/management/v1/projects/_search")
        .match_header("authorization", "Bearer test-token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"result":[{"id":"project-1"}]}"#)
        .create_async()
        .await;
    let create_admin = server
        .mock("POST", "/v2/users/human")
        .match_header("authorization", "Bearer test-token")
        .match_body(Matcher::PartialJson(serde_json::json!({
            "username": "admin",
            "profile": {
                "givenName": "Admin",
                "familyName": "User"
            },
            "email": {
                "email": "admin@example.com",
                "isVerified": true
            }
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"userId":"user-admin"}"#)
        .create_async()
        .await;
    let create_user = server
        .mock("POST", "/v2/users/human")
        .match_header("authorization", "Bearer test-token")
        .match_body(Matcher::PartialJson(serde_json::json!({
            "username": "user",
            "profile": {
                "givenName": "Regular",
                "familyName": "User"
            },
            "email": {
                "email": "user@example.com",
                "isVerified": true
            }
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"userId":"user-regular"}"#)
        .create_async()
        .await;
    let grant_admin = server
        .mock("POST", "/admin/v1/members")
        .match_header("authorization", "Bearer test-token")
        .match_body(Matcher::PartialJson(serde_json::json!({
            "userId": "user-admin",
            "roles": ["IAM_OWNER"]
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{}"#)
        .create_async()
        .await;
    let _apps = server
        .mock("POST", "/management/v1/projects/project-1/apps/_search")
        .match_header("authorization", "Bearer test-token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"result":[]}"#)
        .create_async()
        .await;
    let _users = server
        .mock("POST", "/management/v1/users/_search")
        .match_header("authorization", "Bearer test-token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"result":[]}"#)
        .create_async()
        .await;
    let _idps = server
        .mock("POST", "/admin/v1/idps/_search")
        .match_header("authorization", "Bearer test-token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"result":[]}"#)
        .create_async()
        .await;

    let mut conductor = test_conductor(&server.url());
    conductor.templates.users = vec![
        UserTemplate {
            email: "admin@example.com".to_string(),
            first_name: "Admin".to_string(),
            last_name: "User".to_string(),
            admin: true,
        },
        UserTemplate {
            email: "user@example.com".to_string(),
            first_name: "Regular".to_string(),
            last_name: "User".to_string(),
            admin: false,
        },
    ];
    conductor.client = Some(ZitadelClient::new(server.url(), "test-token".to_string()).unwrap());
    let form = FormState {
        title: "Quick setup users".to_string(),
        description: String::new(),
        submit_label: String::new(),
        fields: vec![
            checkbox_field("template_user", "admin@example.com", true, ""),
            checkbox_field("template_user", "user@example.com", true, ""),
        ],
        selected_field: 0,
        pending: PendingAction::QuickSetupUsers,
    };

    let mode = conductor.quick_setup_users(&form).await;

    create_admin.assert_async().await;
    create_user.assert_async().await;
    grant_admin.assert_async().await;
    assert!(matches!(mode, CanvasMode::Success(_)));
}

#[test]
fn map_app_record_extracts_oidc_fields() {
    let app = serde_json::json!({
        "id": "app-1",
        "name": "grafana",
        "state": "ACTIVE",
        "oidcConfig": {
            "authMethodType": "OIDC_AUTH_METHOD_TYPE_NONE",
            "clientId": "cid-1",
            "redirectUris": ["https://grafana.example.com/callback", "https://grafana.example.com/silent"]
        }
    });
    let record = map_app_record(app);
    assert_eq!(record.id, "app-1");
    assert_eq!(record.name, "grafana");
    assert_eq!(record.kind, "public");
    assert_eq!(record.detail, "cid-1");
    assert_eq!(record.summary, "2 redirects");
    assert_eq!(record.changed_at, "ACTIVE");
}

#[test]
fn map_app_record_confidential_auth_method() {
    let app = serde_json::json!({
        "id": "app-2",
        "name": "vault",
        "state": "INACTIVE",
        "oidcConfig": {
            "authMethodType": "OIDC_AUTH_METHOD_TYPE_BASIC",
            "clientId": "cid-2",
            "redirectUris": []
        }
    });
    let record = map_app_record(app);
    assert_eq!(record.kind, "confidential");
    assert_eq!(record.summary, "0 redirects");
}

#[test]
fn map_app_record_missing_oidc_config() {
    let app = serde_json::json!({"id": "app-3", "name": "plain"});
    let record = map_app_record(app);
    assert_eq!(record.kind, "unknown");
    assert_eq!(record.detail, "missing-client-id");
    assert_eq!(record.summary, "0 redirects");
    assert_eq!(record.changed_at, "unknown");
}

#[test]
fn map_user_record_extracts_human_fields() {
    let user = serde_json::json!({
        "id": "user-1",
        "userName": "admin",
        "state": "ACTIVE",
        "human": {
            "email": {"email": "admin@example.com"},
            "profile": {"displayName": "Admin User"}
        }
    });
    let record = map_user_record(user);
    assert_eq!(record.id, "user-1");
    assert_eq!(record.name, "admin");
    assert_eq!(record.kind, "ACTIVE");
    assert_eq!(record.summary, "admin@example.com");
    assert_eq!(record.detail, "Admin User");
    assert_eq!(record.changed_at, "loaded");
}

#[test]
fn map_user_record_missing_human_fields() {
    let user = serde_json::json!({"id": "user-2", "userName": "bot"});
    let record = map_user_record(user);
    assert_eq!(record.summary, "no email");
    assert_eq!(record.detail, "human user");
}

#[test]
fn map_idp_record_extracts_fields() {
    let idp = serde_json::json!({
        "id": "idp-1",
        "name": "Google",
        "state": "ACTIVE",
        "type": "google"
    });
    let record = map_idp_record(idp);
    assert_eq!(record.id, "idp-1");
    assert_eq!(record.name, "Google");
    assert_eq!(record.kind, "ACTIVE");
    assert_eq!(record.summary, "google");
    assert_eq!(record.detail, "manual credentials");
    assert_eq!(record.changed_at, "configured");
}

#[test]
fn map_idp_record_missing_fields() {
    let idp = serde_json::json!({});
    let record = map_idp_record(idp);
    assert_eq!(record.id, "missing-id");
    assert_eq!(record.name, "unnamed-idp");
    assert_eq!(record.kind, "unknown");
    assert_eq!(record.summary, "provider");
}

#[test]
fn error_mode_creates_error_canvas() {
    let mode = error_mode("title", "message");
    match mode {
        CanvasMode::Error(state) => {
            assert_eq!(state.title, "title");
            assert_eq!(state.lines, vec!["message".to_string()]);
        }
        _ => panic!("expected CanvasMode::Error"),
    }
}

#[test]
fn form_value_finds_matching_key() {
    let form = FormState {
        title: String::new(),
        description: String::new(),
        submit_label: String::new(),
        fields: vec![
            text_field("host", "Host", "https://z.example.com", ""),
            text_field("project", "Project", "proj-1", ""),
        ],
        selected_field: 0,
        pending: PendingAction::SaveConfig,
    };
    assert_eq!(form_value(&form, "host"), "https://z.example.com");
    assert_eq!(form_value(&form, "project"), "proj-1");
    assert_eq!(form_value(&form, "missing"), "");
}

#[test]
fn optional_value_returns_none_for_empty() {
    let form = FormState {
        title: String::new(),
        description: String::new(),
        submit_label: String::new(),
        fields: vec![
            text_field("filled", "Filled", "value", ""),
            text_field("empty", "Empty", "", ""),
            text_field("spaces", "Spaces", "   ", ""),
        ],
        selected_field: 0,
        pending: PendingAction::SaveConfig,
    };
    assert_eq!(optional_value(&form, "filled"), Some("value".to_string()));
    assert_eq!(optional_value(&form, "empty"), None);
    assert_eq!(optional_value(&form, "spaces"), None);
}

#[test]
fn bool_value_recognizes_truthy_values() {
    let make_form = |val: &str| FormState {
        title: String::new(),
        description: String::new(),
        submit_label: String::new(),
        fields: vec![text_field("flag", "Flag", val, "")],
        selected_field: 0,
        pending: PendingAction::SaveConfig,
    };
    assert!(bool_value(&make_form("true"), "flag"));
    assert!(bool_value(&make_form("1"), "flag"));
    assert!(bool_value(&make_form("yes"), "flag"));
    assert!(bool_value(&make_form("on"), "flag"));
    assert!(!bool_value(&make_form("false"), "flag"));
    assert!(!bool_value(&make_form(""), "flag"));
    assert!(!bool_value(&make_form("maybe"), "flag"));
}

#[test]
fn split_csv_empty_string() {
    assert!(split_csv("").is_empty());
}

#[test]
fn split_csv_single_value() {
    assert_eq!(split_csv("hello"), vec!["hello".to_string()]);
}

#[test]
fn app_creation_summary_with_client_id() {
    let value = serde_json::json!({"clientId": "cid-123"});
    assert_eq!(
        app_creation_summary("grafana", &value),
        "grafana -> cid-123"
    );
}

#[test]
fn app_creation_summary_missing_client_id() {
    let value = serde_json::json!({});
    assert_eq!(
        app_creation_summary("grafana", &value),
        "grafana -> created"
    );
}

#[test]
fn checkbox_enabled_truthy_values() {
    assert!(checkbox_enabled("true"));
    assert!(checkbox_enabled("1"));
    assert!(checkbox_enabled("yes"));
    assert!(checkbox_enabled("on"));
    assert!(!checkbox_enabled("false"));
    assert!(!checkbox_enabled(""));
    assert!(!checkbox_enabled("maybe"));
}

#[test]
fn string_field_extracts_value_or_fallback() {
    let value = serde_json::json!({"name": "test"});
    assert_eq!(string_field(&value, "name", "default"), "test");
    assert_eq!(string_field(&value, "missing", "default"), "default");
}

#[test]
fn text_field_constructor() {
    let field = text_field("key", "Label", "value", "help text");
    assert_eq!(field.key, "key");
    assert_eq!(field.label, "Label");
    assert_eq!(field.value, "value");
    assert_eq!(field.help, "help text");
    assert!(matches!(field.kind, crate::tui::FieldKind::Text));
}

#[test]
fn secret_field_constructor() {
    let field = secret_field("key", "Label", "secret", "help");
    assert!(matches!(field.kind, crate::tui::FieldKind::Secret));
    assert_eq!(field.value, "secret");
}

#[test]
fn toggle_field_constructor() {
    let field = toggle_field("key", "Label", true, "help");
    assert!(matches!(field.kind, crate::tui::FieldKind::Toggle));
    assert_eq!(field.value, "true");

    let field = toggle_field("key", "Label", false, "help");
    assert_eq!(field.value, "false");
}

#[test]
fn choice_field_constructor() {
    let opts = vec!["a".to_string(), "b".to_string()];
    let field = choice_field("key", "Label", "a", opts.clone(), "help");
    assert!(matches!(field.kind, crate::tui::FieldKind::Choice(_)));
    assert_eq!(field.value, "a");
}

#[test]
fn checkbox_field_constructor() {
    let field = checkbox_field("key", "Label", true, "help");
    assert!(matches!(field.kind, crate::tui::FieldKind::Checkbox));
    assert_eq!(field.value, "true");

    let field = checkbox_field("key", "Label", false, "help");
    assert_eq!(field.value, "false");
}

#[test]
fn begin_action_apps_create_returns_form() {
    let conductor = test_conductor("https://example.com");
    let mode = conductor.begin_action(ResourceKind::Applications, 0, None);
    assert!(matches!(mode, CanvasMode::EditForm(_)));
}

#[test]
fn begin_action_apps_regenerate_without_record_returns_error() {
    let conductor = test_conductor("https://example.com");
    let mode = conductor.begin_action(ResourceKind::Applications, 1, None);
    assert!(matches!(mode, CanvasMode::Error(_)));
}

#[test]
fn begin_action_apps_delete_with_record_returns_confirm() {
    let conductor = test_conductor("https://example.com");
    let record = Record {
        id: "app-1".to_string(),
        name: "grafana".to_string(),
        kind: "public".to_string(),
        summary: String::new(),
        detail: String::new(),
        changed_at: String::new(),
    };
    let mode = conductor.begin_action(ResourceKind::Applications, 2, Some(&record));
    assert!(matches!(mode, CanvasMode::Confirm(_)));
}

#[test]
fn begin_action_users_create_returns_form() {
    let conductor = test_conductor("https://example.com");
    let mode = conductor.begin_action(ResourceKind::Users, 0, None);
    assert!(matches!(mode, CanvasMode::EditForm(_)));
}

#[test]
fn begin_action_users_grant_without_record_returns_error() {
    let conductor = test_conductor("https://example.com");
    let mode = conductor.begin_action(ResourceKind::Users, 2, None);
    assert!(matches!(mode, CanvasMode::Error(_)));
}

#[test]
fn begin_action_idps_configure_google_returns_form() {
    let conductor = test_conductor("https://example.com");
    let mode = conductor.begin_action(ResourceKind::Idps, 0, None);
    assert!(matches!(mode, CanvasMode::EditForm(_)));
}

#[test]
fn begin_action_auth_setup_returns_setup() {
    let conductor = test_conductor("https://example.com");
    let mode = conductor.begin_action(ResourceKind::Auth, 0, None);
    assert!(matches!(mode, CanvasMode::Setup(_)));
}

#[test]
fn begin_action_config_edit_returns_form() {
    let conductor = test_conductor("https://example.com");
    let mode = conductor.begin_action(ResourceKind::Config, 0, None);
    assert!(matches!(mode, CanvasMode::EditForm(_)));
}
