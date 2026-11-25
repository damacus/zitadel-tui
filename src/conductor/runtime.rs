use reqwest::Client as HttpClient;

use crate::{api::client::ZitadelClient, auth::resolve_access_token, tui::TuiBootstrap};

use super::TuiConductor;

impl TuiConductor {
    pub async fn bootstrap(cli: crate::cli::Cli, config: crate::config::AppConfig) -> Self {
        let templates = config.templates().unwrap_or_default();
        let host = cli
            .host
            .clone()
            .or_else(|| config.zitadel_url.clone())
            .unwrap_or_else(|| "https://zitadel.example.com".to_string());
        let mut conductor = Self {
            cli,
            config,
            templates,
            host,
            project: String::new(),
            auth_label: "Setup required".to_string(),
            setup_required: true,
            client: None,
            app_records: vec![],
            user_records: vec![],
            idp_records: vec![],
            organization: Default::default(),
            organizations: Default::default(),
            projects: Default::default(),
            org_error: Default::default(),
            project_error: Default::default(),
            app_error: Default::default(),
            user_error: Default::default(),
            idp_error: None,
        };
        conductor.refresh_runtime().await;
        conductor
    }

    pub fn bootstrap_state(&self) -> TuiBootstrap {
        TuiBootstrap {
            host: self.host.clone(),
            project: self.project.clone(),
            auth_label: self.auth_label.clone(),
            templates_path: self
                .config
                .apps_config_file
                .as_ref()
                .map(|path| path.display().to_string()),
            setup_required: self.setup_required,
            app_records: self.app_records.clone(),
            user_records: self.user_records.clone(),
            idp_records: self.idp_records.clone(),
            organization: self.organization.clone(),
            organizations: self.organizations.clone(),
            projects: self.projects.clone(),
            org_error: self.org_error.clone(),
            project_error: self.project_error.clone(),
            app_error: self.app_error.clone(),
            user_error: self.user_error.clone(),
            idp_error: self.idp_error.clone(),
        }
    }

    pub async fn refresh_runtime(&mut self) {
        self.clear_directory();
        let has_credential = self.cli.token.is_some()
            || self.config.pat.is_some()
            || self.config.service_account_file.is_some()
            || self.cli.service_account_file.is_some()
            || crate::token_cache::TokenCache::load()
                .ok()
                .flatten()
                .map(|cache| !cache.is_expired())
                .unwrap_or(false);
        self.auth_label = if self.config.pat.is_some() || self.cli.token.is_some() {
            "PAT".to_string()
        } else if self.config.service_account_file.is_some()
            || self.cli.service_account_file.is_some()
        {
            "Service account".to_string()
        } else if has_credential {
            "Session token".to_string()
        } else {
            "Setup required".to_string()
        };
        self.setup_required = self.config.zitadel_url.is_none() || !has_credential;

        let http = HttpClient::new();
        let Ok(auth) = resolve_access_token(
            &http,
            &self.host,
            self.cli.token.clone(),
            self.cli.service_account_file.clone(),
            &self.config,
        )
        .await
        else {
            self.client = None;
            self.setup_required = true;
            return;
        };

        self.auth_label = auth.source.to_string();
        if auth.is_oidc_session() {
            self.auth_label = "Session token (OIDC only)".to_string();
            self.client = None;
            self.setup_required = true;
            return;
        }

        self.setup_required = false;

        let Ok(client) = ZitadelClient::new(self.host.clone(), auth.token) else {
            self.client = None;
            self.setup_required = true;
            return;
        };

        self.load_directory(client).await;
    }
}
