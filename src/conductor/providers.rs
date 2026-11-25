use super::{
    forms::{form_value, secret_field, text_field},
    support::error_mode,
    TuiConductor,
};
use crate::{
    api::{client::ZitadelClient, providers::owner},
    tui::{CanvasMode, FormState, MessageState, PendingAction, Record},
};

impl TuiConductor {
    pub(super) async fn provider_records(client: &ZitadelClient) -> anyhow::Result<Vec<Record>> {
        let providers = client.directory_idps().await?;
        if providers.is_empty() {
            return Ok(vec![]);
        }
        let active = client.active_providers().await;
        Ok(providers
            .iter()
            .map(|p| {
                let mut record = Record::from(p);
                let status = match &active {
                    Ok(ids) if ids.contains(&record.id) => "Enabled for sign-in".to_string(),
                    Ok(_) => "Disabled for sign-in".to_string(),
                    Err(e) => format!("Sign-in status unknown ({e})"),
                };
                record.detail = format!("{} · {status}", record.detail);
                record
            })
            .collect())
    }

    pub async fn edit_google_form(&self, record: Option<&Record>) -> CanvasMode {
        let (Some(client), Some(record)) = (&self.client, record) else {
            return error_mode("Edit Google provider", "Select a provider first.");
        };
        let provider = match client.google_provider(&record.id).await {
            Ok(provider) => provider,
            Err(e) => return error_mode("Cannot read Google provider", &e.to_string()),
        };
        let Some(sequence) = provider
            .pointer("/details/sequence")
            .and_then(|v| v.as_str())
        else {
            return error_mode("Cannot edit provider", "Missing provider revision.");
        };
        let scope = owner(&provider).unwrap_or_default();
        CanvasMode::EditForm(FormState {
            title: format!("Edit Google provider · {scope}"),
            description: format!("{} · {} · {}", record.name, record.id, record.detail),
            submit_label: "[Enter] save provider".into(),
            selected_field: 0,
            pending: PendingAction::EditGoogleIdp {
                id: record.id.clone(),
                scope: scope.into(),
                sequence: sequence.into(),
            },
            fields: vec![
                text_field(
                    "name",
                    "Name",
                    provider["name"].as_str().unwrap_or_default(),
                    "Display name",
                ),
                text_field(
                    "client_id",
                    "Client ID",
                    provider["config"]["google"]["clientId"]
                        .as_str()
                        .unwrap_or_default(),
                    "Google OAuth client ID",
                ),
                secret_field(
                    "client_secret",
                    "New secret",
                    "",
                    "Leave blank to keep the current secret.",
                ),
            ],
        })
    }

    pub(super) async fn submit_google(&mut self, form: &FormState) -> CanvasMode {
        let Some(client) = &self.client else {
            return error_mode("Authentication required", "Run setup first.");
        };
        let create_scope = form_value(form, "scope");
        let (scope, existing) = match &form.pending {
            PendingAction::EditGoogleIdp {
                id,
                scope,
                sequence,
            } => (scope.as_str(), Some((id.as_str(), sequence.as_str()))),
            _ => (create_scope.as_str(), None),
        };
        match client
            .save_google(
                existing,
                scope,
                &form_value(form, "name"),
                &form_value(form, "client_id"),
                &form_value(form, "client_secret"),
            )
            .await
        {
            Ok(()) => {
                self.refresh_runtime().await;
                CanvasMode::Success(MessageState {
                    title: "Google provider saved".into(),
                    lines: vec![format!("Scope: {scope}")],
                })
            }
            Err(e) => error_mode("Could not save Google provider", &e.to_string()),
        }
    }
}
