use super::*;
use mockito::{Matcher, Server};

#[tokio::test]
async fn creating_google_requires_an_explicit_scope_and_uses_its_endpoint() {
    let mut server = Server::new_async().await;
    let mut client = ZitadelClient::new(server.url(), "test".into()).unwrap();
    client.organization = Some("org-1".into());
    assert!(client
        .save_google(None, "Choose scope", "Google", "cid", "secret")
        .await
        .is_err());
    let org = server
        .mock("POST", "/management/v1/idps/google")
        .match_header("x-zitadel-orgid", "org-1")
        .with_body(r#"{"id":"org-google"}"#)
        .create_async()
        .await;
    let instance = server
        .mock("POST", "/admin/v1/idps/google")
        .match_header("x-zitadel-orgid", Matcher::Missing)
        .with_body(r#"{"id":"instance-google"}"#)
        .create_async()
        .await;
    for scope in ["Organization", "Instance"] {
        client
            .save_google(None, scope, "Google", "cid", "secret")
            .await
            .unwrap();
    }
    org.assert_async().await;
    instance.assert_async().await;
}

#[tokio::test]
async fn changed_revision_and_foreign_organization_block_edits() {
    let mut server = Server::new_async().await;
    let mut client = ZitadelClient::new(server.url(), "test".into()).unwrap();
    client.organization = Some("org-1".into());
    let read = server.mock("GET","/management/v1/idps/templates/p")
        .with_body(json!({"idp":{"id":"p","owner":"IDP_OWNER_TYPE_ORG","details":{"sequence":"2","resourceOwner":"org-1"},"config":{"google":{"clientId":"cid"}}}}).to_string()).expect(2).create_async().await;
    assert!(client
        .save_google(Some(("p", "1")), "Organization", "Google", "cid", "")
        .await
        .unwrap_err()
        .to_string()
        .contains("changed"));
    client.organization = Some("org-2".into());
    assert!(client
        .save_google(Some(("p", "2")), "Organization", "Google", "cid", "")
        .await
        .unwrap_err()
        .to_string()
        .contains("ownership"));
    read.assert_async().await;
}

#[tokio::test]
async fn edit_preserves_scopes_options_and_existing_secret() {
    let mut server = Server::new_async().await;
    let mut client = ZitadelClient::new(server.url(), "test".into()).unwrap();
    client.organization = Some("org-1".into());
    let read = server.mock("GET","/management/v1/idps/templates/google-1")
        .match_header("x-zitadel-orgid","org-1")
        .with_body(json!({"idp":{"id":"google-1","owner":"IDP_OWNER_TYPE_SYSTEM","details":{"sequence":"9"},"config":{"google":{"clientId":"old","scopes":["email"]},"options":{"autoLinking":"AUTO_LINKING_OPTION_EMAIL"}}}}).to_string()).create_async().await;
    let write = server.mock("PUT","/admin/v1/idps/google/google-1")
        .match_body(Matcher::Json(json!({"name":"New","clientId":"new","scopes":["email"],"providerOptions":{"autoLinking":"AUTO_LINKING_OPTION_EMAIL"}}))).create_async().await;
    client
        .save_google(Some(("google-1", "9")), "Instance", "New", "new", "")
        .await
        .unwrap();
    read.assert_async().await;
    write.assert_async().await;
}

#[tokio::test]
async fn checks_effective_sign_in_for_selected_org_and_reports_denied_reads() {
    let mut server = Server::new_async().await;
    let mut client = ZitadelClient::new(server.url(), "test".into()).unwrap();
    client.organization = Some("org-1".into());
    let _settings = server
        .mock("GET", "/v2/settings/login?ctx.orgId=org-1")
        .with_body(r#"{"settings":{"allowExternalIdp":true}}"#)
        .create_async()
        .await;
    let active = server
        .mock("GET", "/v2/settings/login/idps?ctx.orgId=org-1")
        .with_body(r#"{"identityProviders":[{"id":"p1"}]}"#)
        .create_async()
        .await;
    assert_eq!(client.active_providers().await.unwrap(), ["p1"]);
    active.remove_async().await;
    let _denied = server
        .mock("GET", "/v2/settings/login/idps?ctx.orgId=org-1")
        .with_status(403)
        .create_async()
        .await;
    assert!(client
        .active_providers()
        .await
        .unwrap_err()
        .to_string()
        .contains("403"));
}
