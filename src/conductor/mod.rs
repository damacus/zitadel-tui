mod forms;
mod mutations;
mod records;
mod runtime;
mod support;
#[cfg(test)]
mod tests;

use crate::{
    cli::Cli,
    client::ZitadelClient,
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
}
