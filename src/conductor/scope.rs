use super::TuiConductor;
use crate::{api::client::ZitadelClient, tui::Record};
use anyhow::{bail, Result};

fn section<T>(result: Result<Vec<T>>, label: &str, error: &mut Option<String>) -> Vec<T> {
    *error = result
        .as_ref()
        .err()
        .map(|cause| format!("Could not load {label}: {cause}"));
    result.unwrap_or_default()
}

impl TuiConductor {
    pub(super) fn clear_directory(&mut self) {
        self.client = None;
        self.organizations.clear();
        self.projects.clear();
        self.app_records.clear();
        self.user_records.clear();
        self.idp_records.clear();
        self.org_error = None;
        self.project_error = None;
        self.app_error = None;
        self.user_error = None;
        self.idp_error = None;
    }

    pub async fn select_organization(&mut self, id: &str) -> Result<()> {
        if !self.organizations.iter().any(|(key, _)| key == id) {
            bail!("Organization is unavailable");
        }
        self.organization = Some(id.to_owned());
        self.project.clear();
        self.refresh_runtime().await;
        Ok(())
    }

    pub async fn select_project(&mut self, id: &str) -> Result<()> {
        if !self.projects.iter().any(|(key, _)| key == id) {
            bail!("Project is unavailable");
        }
        self.project = id.to_owned();
        self.refresh_runtime().await;
        Ok(())
    }

    pub(super) async fn load_directory(&mut self, mut client: ZitadelClient) {
        let use_configured_project = self.organization.is_none();
        if self.organization.is_none() {
            if let Ok(identity) = client.whoami().await {
                self.organization = identity
                    .raw
                    .pointer("/user/details/resourceOwner")
                    .and_then(|id| id.as_str())
                    .map(str::to_owned);
            }
        }
        client.organization.clone_from(&self.organization);
        let (organizations, projects, users, providers) = tokio::join!(
            client.directory_organizations(),
            client.directory_projects(),
            client.directory_users(),
            Self::provider_records(&client),
        );
        self.organizations = section(organizations, "organizations", &mut self.org_error);
        self.projects = section(projects, "projects", &mut self.project_error);
        self.user_records = section(users, "users", &mut self.user_error)
            .iter()
            .map(Record::from)
            .collect();
        self.idp_records = section(providers, "identity providers", &mut self.idp_error);
        let configured = self
            .cli
            .project_id
            .as_ref()
            .or(self.config.project_id.as_ref());
        // Explicit project choices survive refresh failures and permission changes.
        // An organization switch clears the previous project before loading its list.
        if self.project.is_empty() {
            let initial = if use_configured_project {
                configured.cloned()
            } else {
                None
            };
            self.project = initial
                .or_else(|| self.projects.first().map(|(id, _)| id.clone()))
                .unwrap_or_default();
        }
        if self.project_error.is_none()
            && self.organization.is_some()
            && !self.projects.iter().any(|(id, _)| id == &self.project)
        {
            self.project.clear();
        }
        if self.project.is_empty() {
            self.app_error = Some(
                self.project_error
                    .clone()
                    .unwrap_or_else(|| "No project available".to_owned()),
            );
        } else {
            self.app_records = section(
                client.directory_apps(&self.project).await,
                "applications",
                &mut self.app_error,
            )
            .iter()
            .map(Record::from)
            .collect();
        }
        self.client = Some(client);
    }
}
