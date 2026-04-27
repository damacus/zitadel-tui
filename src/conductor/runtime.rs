use anyhow::{Context, Result};
use reqwest::Client as HttpClient;

use crate::{auth::resolve_access_token, client::ZitadelClient, tui::TuiBootstrap};

use super::{
    records::{map_app_record, map_idp_record, map_user_record},
    TuiConductor,
};

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
        }
    }

    pub async fn refresh_runtime(&mut self) {
        self.project = self
            .cli
            .project_id
            .clone()
            .or_else(|| self.config.project_id.clone())
            .unwrap_or_else(|| "default".to_string());

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
            return;
        };

        self.auth_label = auth.source.to_string();
        self.setup_required = false;

        let Ok(client) = ZitadelClient::new(self.host.clone(), auth.token) else {
            self.client = None;
            self.setup_required = true;
            return;
        };

        let Ok(project_id) = resolve_project_id(
            &client,
            self.cli.project_id.clone(),
            self.config.project_id.clone(),
        )
        .await
        else {
            self.client = Some(client);
            return;
        };

        self.project = project_id.clone();
        let (apps, users, idps) = tokio::join!(
            client.list_apps(&project_id),
            client.list_users(100),
            client.list_idps()
        );
        self.app_records = apps
            .map(|apps| apps.into_iter().map(map_app_record).collect())
            .unwrap_or_default();
        self.user_records = users
            .map(|users| users.into_iter().map(map_user_record).collect())
            .unwrap_or_default();
        self.idp_records = idps
            .map(|idps| idps.into_iter().map(map_idp_record).collect())
            .unwrap_or_default();
        self.client = Some(client);
    }
}

pub(crate) async fn resolve_project_id(
    client: &ZitadelClient,
    cli_project: Option<String>,
    config_project: Option<String>,
) -> Result<String> {
    if let Some(project) = cli_project.or(config_project) {
        return Ok(project);
    }

    client
        .get_default_project()
        .await?
        .get("id")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
        .context("failed to determine default project id")
}
