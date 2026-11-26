mod forms;
mod mutations;
mod providers;
mod records;
mod runtime;
mod scope;
mod support;
#[cfg(test)]
mod tests;
mod user_actions;

use crate::{
    api::client::ZitadelClient,
    cli::Cli,
    config::{AppConfig, TemplatesFile},
    tui::Record,
};

pub struct TuiConductor {
    cli: Cli,
    pub config: AppConfig,
    templates: TemplatesFile,
    host: String,
    project: String,
    auth_label: String,
    setup_required: bool,
    client: Option<ZitadelClient>,
    app_records: Vec<Record>,
    user_records: Vec<Record>,
    idp_records: Vec<Record>,
    organization: Option<String>,
    organizations: Vec<(String, String)>,
    projects: Vec<(String, String)>,
    org_error: Option<String>,
    project_error: Option<String>,
    app_error: Option<String>,
    user_error: Option<String>,
    idp_error: Option<String>,
}
