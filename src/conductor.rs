use std::path::PathBuf;

use anyhow::{Context, Result};
use reqwest::Client as HttpClient;
use serde_json::Value;

use crate::{
    auth::resolve_access_token,
    cli::Cli,
    client::ZitadelClient,
    config::{AppConfig, AppTemplate, TemplatesFile, UserTemplate},
    tui::{
        CanvasMode, ConfirmState, FormField, FormState, MessageState, PendingAction, Record,
        ResourceKind, TuiBootstrap,
    },
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

impl TuiConductor {
    pub async fn bootstrap(cli: Cli, config: AppConfig) -> Self {
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

        self.auth_label = if self.config.pat.is_some() {
            "PAT".to_string()
        } else if self.config.service_account_file.is_some() {
            "Service account".to_string()
        } else {
            "Setup required".to_string()
        };
        self.setup_required = self.config.zitadel_url.is_none()
            || (self.config.pat.is_none() && self.config.service_account_file.is_none());

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

    pub fn begin_action(
        &self,
        resource: ResourceKind,
        action_index: usize,
        selected_record: Option<&Record>,
    ) -> CanvasMode {
        match resource {
            ResourceKind::Applications => match action_index {
                0 => CanvasMode::EditForm(FormState {
                    title: "Create application".to_string(),
                    description: "Create a new OIDC application from a template or manually."
                        .to_string(),
                    submit_label: "[Enter] create application".to_string(),
                    selected_field: 0,
                    pending: PendingAction::CreateApplication,
                    fields: vec![
                        choice_field(
                            "mode",
                            "Mode",
                            "manual",
                            vec!["manual".to_string(), "template".to_string()],
                            "Choose manual fields or template-driven defaults.",
                        ),
                        text_field("name", "Name", "", "Application name"),
                        text_field(
                            "redirect_uris",
                            "Redirect URIs",
                            "",
                            "Comma-separated callback URLs",
                        ),
                        toggle_field("public", "Public", false, "Public clients have no secret"),
                        text_field(
                            "template",
                            "Template",
                            "",
                            "Template name when mode is template",
                        ),
                    ],
                }),
                1 => {
                    if let Some(record) = selected_record {
                        CanvasMode::Confirm(ConfirmState {
                            title: "Regenerate client secret".to_string(),
                            lines: vec![
                                format!("Application: {}", record.name),
                                "This invalidates the current confidential client secret."
                                    .to_string(),
                            ],
                            submit_label: "[Enter] regenerate secret".to_string(),
                            pending: PendingAction::RegenerateSecret {
                                app_id: record.id.clone(),
                                name: record.name.clone(),
                                client_id: record.detail.clone(),
                            },
                        })
                    } else {
                        error_mode("No application selected", "Choose an application first.")
                    }
                }
                2 => {
                    if let Some(record) = selected_record {
                        CanvasMode::Confirm(ConfirmState {
                            title: "Delete application".to_string(),
                            lines: vec![
                                format!("Application: {}", record.name),
                                "This permanently deletes the selected application.".to_string(),
                            ],
                            submit_label: "[Enter] delete application".to_string(),
                            pending: PendingAction::DeleteApplication {
                                app_id: record.id.clone(),
                                name: record.name.clone(),
                            },
                        })
                    } else {
                        error_mode("No application selected", "Choose an application first.")
                    }
                }
                3 => CanvasMode::EditForm(FormState {
                    title: "Quick setup applications".to_string(),
                    description: "Select predefined application templates to create.".to_string(),
                    submit_label: "[Enter] create selected apps".to_string(),
                    selected_field: 0,
                    pending: PendingAction::QuickSetupApplications,
                    fields: self
                        .templates
                        .apps
                        .keys()
                        .map(|name| {
                            checkbox_field(
                                "template_app",
                                name,
                                false,
                                "Toggle application creation",
                            )
                        })
                        .collect(),
                }),
                _ => CanvasMode::Browse,
            },
            ResourceKind::Users => match action_index {
                0 => CanvasMode::EditForm(FormState {
                    title: "Create user".to_string(),
                    description: "Create a normal human user.".to_string(),
                    submit_label: "[Enter] create user".to_string(),
                    selected_field: 0,
                    pending: PendingAction::CreateUser,
                    fields: vec![
                        text_field("email", "Email", "", "Email address"),
                        text_field("first_name", "First name", "", "Given name"),
                        text_field("last_name", "Last name", "", "Family name"),
                        text_field("username", "Username", "", "Optional username override"),
                    ],
                }),
                1 => CanvasMode::EditForm(FormState {
                    title: "Create admin user".to_string(),
                    description: "Create a local admin import with temporary password.".to_string(),
                    submit_label: "[Enter] create admin".to_string(),
                    selected_field: 0,
                    pending: PendingAction::CreateAdminUser,
                    fields: vec![
                        text_field("username", "Username", "admin", "Login username"),
                        text_field("first_name", "First name", "Admin", "Given name"),
                        text_field("last_name", "Last name", "User", "Family name"),
                        text_field("email", "Email", "", "Email address"),
                        secret_field("password", "Password", "", "Temporary password"),
                    ],
                }),
                2 => {
                    if let Some(record) = selected_record {
                        CanvasMode::Confirm(ConfirmState {
                            title: "Grant IAM_OWNER".to_string(),
                            lines: vec![
                                format!("User: {}", record.name),
                                "This grants full instance administration rights.".to_string(),
                            ],
                            submit_label: "[Enter] grant IAM_OWNER".to_string(),
                            pending: PendingAction::GrantIamOwner {
                                user_id: record.id.clone(),
                                username: record.name.clone(),
                            },
                        })
                    } else {
                        error_mode("No user selected", "Choose a user first.")
                    }
                }
                3 => CanvasMode::EditForm(FormState {
                    title: "Quick setup users".to_string(),
                    description: "Select predefined users to create.".to_string(),
                    submit_label: "[Enter] create selected users".to_string(),
                    selected_field: 0,
                    pending: PendingAction::QuickSetupUsers,
                    fields: self
                        .templates
                        .users
                        .iter()
                        .map(|user| {
                            checkbox_field(
                                "template_user",
                                &format!(
                                    "{} ({}){}",
                                    user.email,
                                    user.first_name,
                                    if user.admin { " admin" } else { "" }
                                ),
                                false,
                                "Toggle user creation",
                            )
                        })
                        .collect(),
                }),
                _ => CanvasMode::Browse,
            },
            ResourceKind::Idps => CanvasMode::EditForm(FormState {
                title: "Configure Google IDP".to_string(),
                description: "Configure Google with manual credentials only.".to_string(),
                submit_label: "[Enter] configure Google".to_string(),
                selected_field: 0,
                pending: PendingAction::ConfigureGoogleIdp,
                fields: vec![
                    text_field("name", "Name", "Google", "Display name"),
                    text_field("client_id", "Client ID", "", "Google OAuth client ID"),
                    secret_field(
                        "client_secret",
                        "Client secret",
                        "",
                        "Google OAuth client secret",
                    ),
                ],
            }),
            ResourceKind::Auth => CanvasMode::Setup(FormState {
                title: "Run setup".to_string(),
                description: "Validate and save auth, host, project, and templates path."
                    .to_string(),
                submit_label: "[Enter] validate and save".to_string(),
                selected_field: 0,
                pending: PendingAction::ValidateAuthSetup,
                fields: vec![
                    text_field("host", "Host", &self.host, "Zitadel base URL"),
                    text_field(
                        "project",
                        "Project",
                        &self.project,
                        "Optional default project",
                    ),
                    choice_field(
                        "auth_method",
                        "Auth method",
                        if self.config.service_account_file.is_some() {
                            "Service account"
                        } else {
                            "PAT"
                        },
                        vec![
                            "PAT".to_string(),
                            "Service account".to_string(),
                            "OAuth device (placeholder)".to_string(),
                        ],
                        "PAT and service account are live in this slice.",
                    ),
                    secret_field(
                        "token",
                        "PAT",
                        self.config.pat.as_deref().unwrap_or(""),
                        "Used when auth method is PAT",
                    ),
                    text_field(
                        "service_account_file",
                        "Service account",
                        &self
                            .config
                            .service_account_file
                            .as_ref()
                            .map(|path| path.display().to_string())
                            .unwrap_or_default(),
                        "Used when auth method is service account",
                    ),
                    text_field(
                        "templates_path",
                        "Templates path",
                        &self
                            .config
                            .apps_config_file
                            .as_ref()
                            .map(|path| path.display().to_string())
                            .unwrap_or_default(),
                        "Optional apps/users YAML path",
                    ),
                ],
            }),
            ResourceKind::Config => match action_index {
                0 => CanvasMode::EditForm(FormState {
                    title: "Edit config".to_string(),
                    description: "Update saved host, project, and templates path.".to_string(),
                    submit_label: "[Enter] save config".to_string(),
                    selected_field: 0,
                    pending: PendingAction::SaveConfig,
                    fields: vec![
                        text_field("host", "Host", &self.host, "Saved Zitadel URL"),
                        text_field("project", "Project", &self.project, "Saved default project"),
                        text_field(
                            "templates_path",
                            "Templates path",
                            &self
                                .config
                                .apps_config_file
                                .as_ref()
                                .map(|path| path.display().to_string())
                                .unwrap_or_default(),
                            "Saved templates YAML path",
                        ),
                    ],
                }),
                _ => CanvasMode::Browse,
            },
        }
    }

    pub async fn submit_form(&mut self, form: &FormState) -> CanvasMode {
        match &form.pending {
            PendingAction::CreateApplication => self.create_application(form).await,
            PendingAction::QuickSetupApplications => self.quick_setup_apps(form).await,
            PendingAction::CreateUser => self.create_user(form).await,
            PendingAction::CreateAdminUser => self.create_admin_user(form).await,
            PendingAction::QuickSetupUsers => self.quick_setup_users(form).await,
            PendingAction::ConfigureGoogleIdp => self.configure_google(form).await,
            PendingAction::ValidateAuthSetup => self.validate_and_save_setup(form).await,
            PendingAction::SaveConfig => self.save_config_form(form).await,
            PendingAction::DeleteApplication { .. }
            | PendingAction::RegenerateSecret { .. }
            | PendingAction::GrantIamOwner { .. } => error_mode(
                "Invalid form state",
                "This action requires confirmation instead.",
            ),
        }
    }

    pub async fn confirm(&mut self, pending: PendingAction) -> CanvasMode {
        match pending {
            PendingAction::DeleteApplication { app_id, name } => match self.client.as_ref() {
                Some(client) => {
                    let result = client.delete_app(&self.project, &app_id).await;
                    self.finish_simple_mutation(
                        result,
                        "Application deleted",
                        vec![format!("Deleted application {name}.")],
                    )
                    .await
                }
                None => error_mode("Authentication required", "Run setup first."),
            },
            PendingAction::RegenerateSecret {
                app_id,
                name,
                client_id,
            } => match self.client.as_ref() {
                Some(client) => {
                    let result = client.regenerate_secret(&self.project, &app_id).await;
                    match result {
                        Ok(value) => {
                            self.refresh_runtime().await;
                            CanvasMode::Success(MessageState {
                                title: "Secret regenerated".to_string(),
                                lines: vec![
                                    format!("Application: {name}"),
                                    format!("Client ID: {client_id}"),
                                    format!(
                                        "Client Secret: {}",
                                        value
                                            .get("clientSecret")
                                            .and_then(|item| item.as_str())
                                            .unwrap_or("missing")
                                    ),
                                    "This secret is only shown once.".to_string(),
                                ],
                            })
                        }
                        Err(error) => error_mode("Failed to regenerate secret", &error.to_string()),
                    }
                }
                None => error_mode("Authentication required", "Run setup first."),
            },
            PendingAction::GrantIamOwner { user_id, username } => match self.client.as_ref() {
                Some(client) => {
                    let result = client.grant_iam_owner(&user_id).await;
                    self.finish_simple_mutation(
                        result,
                        "IAM_OWNER granted",
                        vec![
                            format!("Granted IAM_OWNER to {username}."),
                            "Log out and back in to see role changes in the console.".to_string(),
                        ],
                    )
                    .await
                }
                None => error_mode("Authentication required", "Run setup first."),
            },
            PendingAction::CreateApplication
            | PendingAction::QuickSetupApplications
            | PendingAction::CreateUser
            | PendingAction::CreateAdminUser
            | PendingAction::QuickSetupUsers
            | PendingAction::ConfigureGoogleIdp
            | PendingAction::ValidateAuthSetup
            | PendingAction::SaveConfig => error_mode(
                "Invalid confirmation state",
                "This action requires form submission instead.",
            ),
        }
    }

    async fn finish_simple_mutation(
        &mut self,
        result: Result<Value>,
        title: &str,
        lines: Vec<String>,
    ) -> CanvasMode {
        match result {
            Ok(_) => {
                self.refresh_runtime().await;
                CanvasMode::Success(MessageState {
                    title: title.to_string(),
                    lines,
                })
            }
            Err(error) => error_mode(&format!("{title} failed"), &error.to_string()),
        }
    }

    async fn create_application(&mut self, form: &FormState) -> CanvasMode {
        let Some(client) = self.client.as_ref() else {
            return error_mode("Authentication required", "Run setup first.");
        };

        let mode = form_value(form, "mode");
        let result = if mode == "template" {
            let template_name = form_value(form, "template");
            let Some(template) = self.templates.apps.get(&template_name) else {
                return error_mode("Template not found", "Choose a valid application template.");
            };
            let manual_name = form_value(form, "name");
            let resolved_name = if manual_name.is_empty() {
                template_name.clone()
            } else {
                manual_name
            };
            client
                .create_oidc_app(
                    &self.project,
                    &resolved_name,
                    template.redirect_uris.clone(),
                    template.public,
                )
                .await
        } else {
            let name = form_value(form, "name");
            let redirect_uris = split_csv(&form_value(form, "redirect_uris"));
            if name.is_empty() || redirect_uris.is_empty() {
                return error_mode(
                    "Missing application data",
                    "Name and redirect URIs are required for manual application creation.",
                );
            }
            client
                .create_oidc_app(
                    &self.project,
                    &name,
                    redirect_uris,
                    bool_value(form, "public"),
                )
                .await
        };

        match result {
            Ok(value) => {
                self.refresh_runtime().await;
                let mut lines = vec![format!(
                    "Client ID: {}",
                    value
                        .get("clientId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("missing")
                )];
                if let Some(secret) = value.get("clientSecret").and_then(|v| v.as_str()) {
                    lines.push(format!("Client Secret: {secret}"));
                    lines.push("This secret is only shown once.".to_string());
                }
                CanvasMode::Success(MessageState {
                    title: "Application created".to_string(),
                    lines,
                })
            }
            Err(error) => error_mode("Create application failed", &error.to_string()),
        }
    }

    async fn quick_setup_apps(&mut self, form: &FormState) -> CanvasMode {
        let Some(client) = self.client.as_ref() else {
            return error_mode("Authentication required", "Run setup first.");
        };
        let selected_templates: Vec<(String, AppTemplate)> = form
            .fields
            .iter()
            .filter(|field| checkbox_enabled(&field.value))
            .filter_map(|field| {
                self.templates
                    .apps
                    .get(&field.label)
                    .cloned()
                    .map(|template| (field.label.clone(), template))
            })
            .collect();

        let mut created = Vec::new();
        for chunk in selected_templates.chunks(2) {
            match chunk {
                [first] => match client
                    .create_oidc_app(
                        &self.project,
                        &first.0,
                        first.1.redirect_uris.clone(),
                        first.1.public,
                    )
                    .await
                {
                    Ok(value) => created.push(app_creation_summary(&first.0, &value)),
                    Err(error) => {
                        return error_mode(
                            "Quick setup failed",
                            &format!("{} failed: {}", first.0, error),
                        )
                    }
                },
                [first, second] => {
                    let first_name = first.0.clone();
                    let first_template = first.1.clone();
                    let second_name = second.0.clone();
                    let second_template = second.1.clone();
                    let (first_result, second_result) = tokio::join!(
                        client.create_oidc_app(
                            &self.project,
                            &first_name,
                            first_template.redirect_uris.clone(),
                            first_template.public,
                        ),
                        client.create_oidc_app(
                            &self.project,
                            &second_name,
                            second_template.redirect_uris.clone(),
                            second_template.public,
                        ),
                    );
                    let first_value = match first_result {
                        Ok(value) => value,
                        Err(error) => {
                            return error_mode(
                                "Quick setup failed",
                                &format!("{} failed: {}", first_name, error),
                            )
                        }
                    };
                    let second_value = match second_result {
                        Ok(value) => value,
                        Err(error) => {
                            return error_mode(
                                "Quick setup failed",
                                &format!("{} failed: {}", second_name, error),
                            )
                        }
                    };
                    created.push(app_creation_summary(&first_name, &first_value));
                    created.push(app_creation_summary(&second_name, &second_value));
                }
                _ => unreachable!("chunks(2) yields at most two items"),
            }
        }
        self.refresh_runtime().await;
        CanvasMode::Success(MessageState {
            title: "Quick setup complete".to_string(),
            lines: if created.is_empty() {
                vec!["No application templates were selected.".to_string()]
            } else {
                created
            },
        })
    }

    async fn create_user(&mut self, form: &FormState) -> CanvasMode {
        let Some(client) = self.client.as_ref() else {
            return error_mode("Authentication required", "Run setup first.");
        };
        let email = form_value(form, "email");
        let first_name = form_value(form, "first_name");
        let last_name = form_value(form, "last_name");
        if email.is_empty() || first_name.is_empty() || last_name.is_empty() {
            return error_mode(
                "Missing user data",
                "Email, first name, and last name are required.",
            );
        }

        match client
            .create_human_user(
                &email,
                &first_name,
                &last_name,
                optional_value(form, "username").as_deref(),
            )
            .await
        {
            Ok(value) => {
                self.refresh_runtime().await;
                CanvasMode::Success(MessageState {
                    title: "User created".to_string(),
                    lines: vec![format!(
                        "User ID: {}",
                        value
                            .get("userId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("missing")
                    )],
                })
            }
            Err(error) => error_mode("Create user failed", &error.to_string()),
        }
    }

    async fn create_admin_user(&mut self, form: &FormState) -> CanvasMode {
        let Some(client) = self.client.as_ref() else {
            return error_mode("Authentication required", "Run setup first.");
        };
        let username = form_value(form, "username");
        let first_name = form_value(form, "first_name");
        let last_name = form_value(form, "last_name");
        let email = form_value(form, "email");
        let password = form_value(form, "password");
        if username.is_empty()
            || first_name.is_empty()
            || last_name.is_empty()
            || email.is_empty()
            || password.is_empty()
        {
            return error_mode(
                "Missing admin data",
                "Username, names, email, and temporary password are required.",
            );
        }

        match client
            .import_human_user(&username, &first_name, &last_name, &email, &password, true)
            .await
        {
            Ok(_) => {
                self.refresh_runtime().await;
                CanvasMode::Success(MessageState {
                    title: "Admin user created".to_string(),
                    lines: vec![
                        format!("{}/ui/console/", self.host.trim_end_matches('/')),
                        format!("Username: {username}"),
                        format!("Password: {password}"),
                        "Password change will be required on first login.".to_string(),
                    ],
                })
            }
            Err(error) => error_mode("Create admin failed", &error.to_string()),
        }
    }

    async fn quick_setup_users(&mut self, form: &FormState) -> CanvasMode {
        let Some(client) = self.client.as_ref() else {
            return error_mode("Authentication required", "Run setup first.");
        };
        let selected_users: Vec<UserTemplate> = form
            .fields
            .iter()
            .enumerate()
            .filter(|(_, field)| checkbox_enabled(&field.value))
            .filter_map(|(index, _)| self.templates.users.get(index).cloned())
            .collect();

        let mut lines = Vec::new();
        for chunk in selected_users.chunks(2) {
            match chunk {
                [first] => match create_quick_user(client, first.clone()).await {
                    Ok(mut created_lines) => lines.append(&mut created_lines),
                    Err(error) => {
                        return error_mode(
                            "Quick setup failed",
                            &format!("Failed to create {}: {}", first.email, error),
                        )
                    }
                },
                [first, second] => {
                    let first_user = first.clone();
                    let second_user = second.clone();
                    let (first_result, second_result) = tokio::join!(
                        create_quick_user(client, first_user),
                        create_quick_user(client, second_user),
                    );
                    match first_result {
                        Ok(mut created_lines) => lines.append(&mut created_lines),
                        Err(error) => {
                            return error_mode(
                                "Quick setup failed",
                                &format!("Failed to create {}: {}", first.email, error),
                            )
                        }
                    }
                    match second_result {
                        Ok(mut created_lines) => lines.append(&mut created_lines),
                        Err(error) => {
                            return error_mode(
                                "Quick setup failed",
                                &format!("Failed to create {}: {}", second.email, error),
                            )
                        }
                    }
                }
                _ => unreachable!("chunks(2) yields at most two items"),
            }
        }
        self.refresh_runtime().await;
        CanvasMode::Success(MessageState {
            title: "Quick setup complete".to_string(),
            lines: if lines.is_empty() {
                vec!["No user templates were selected.".to_string()]
            } else {
                lines
            },
        })
    }

    async fn configure_google(&mut self, form: &FormState) -> CanvasMode {
        let Some(client) = self.client.as_ref() else {
            return error_mode("Authentication required", "Run setup first.");
        };
        let name = form_value(form, "name");
        let client_id = form_value(form, "client_id");
        let client_secret = form_value(form, "client_secret");
        if client_id.is_empty() || client_secret.is_empty() {
            return error_mode(
                "Missing Google credentials",
                "Client ID and client secret are required for Google IDP setup.",
            );
        }
        match client
            .add_google_idp(&client_id, &client_secret, &name)
            .await
        {
            Ok(value) => {
                self.refresh_runtime().await;
                CanvasMode::Success(MessageState {
                    title: "Google IDP configured".to_string(),
                    lines: vec![format!(
                        "IDP ID: {}",
                        value
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("missing")
                    )],
                })
            }
            Err(error) => error_mode("Configure Google failed", &error.to_string()),
        }
    }

    async fn validate_and_save_setup(&mut self, form: &FormState) -> CanvasMode {
        let host = form_value(form, "host");
        if host.is_empty() {
            return error_mode("Missing host", "Host is required.");
        }
        let auth_method = form_value(form, "auth_method");
        if auth_method == "OAuth device (placeholder)" {
            return error_mode(
                "OAuth device flow not implemented",
                "Use PAT or service account in this migration slice.",
            );
        }

        let mut config = self.config.clone();
        config.zitadel_url = Some(host.clone());
        config.project_id = optional_value(form, "project");
        config.apps_config_file = optional_value(form, "templates_path").map(PathBuf::from);
        config.pat = None;
        config.service_account_file = None;

        let cli_token = if auth_method == "PAT" {
            let token = form_value(form, "token");
            if token.is_empty() {
                return error_mode("Missing PAT", "PAT is required when PAT auth is selected.");
            }
            Some(token)
        } else {
            None
        };

        let cli_service_account = if auth_method == "Service account" {
            let path = form_value(form, "service_account_file");
            if path.is_empty() {
                return error_mode(
                    "Missing service account file",
                    "Service account file is required when service-account auth is selected.",
                );
            }
            Some(PathBuf::from(path))
        } else {
            None
        };

        if auth_method == "PAT" {
            config.pat = cli_token.clone();
        } else if let Some(path) = cli_service_account.clone() {
            config.service_account_file = Some(path);
        }

        let http = HttpClient::new();
        match resolve_access_token(&http, &host, cli_token, cli_service_account, &config).await {
            Ok(_) => match config.save_to_canonical_path() {
                Ok(_) => {
                    self.config = config;
                    self.templates = self.config.templates().unwrap_or_default();
                    self.host = host;
                    self.refresh_runtime().await;
                    CanvasMode::Success(MessageState {
                        title: "Setup validated".to_string(),
                        lines: vec![
                            "Credentials validated successfully.".to_string(),
                            "Canonical TOML config was updated.".to_string(),
                        ],
                    })
                }
                Err(error) => error_mode("Failed to save config", &error.to_string()),
            },
            Err(error) => error_mode("Auth validation failed", &error.to_string()),
        }
    }

    async fn save_config_form(&mut self, form: &FormState) -> CanvasMode {
        self.config.zitadel_url = optional_value(form, "host");
        self.config.project_id = optional_value(form, "project");
        self.config.apps_config_file = optional_value(form, "templates_path").map(PathBuf::from);
        match self.config.save_to_canonical_path() {
            Ok(path) => {
                self.templates = self.config.templates().unwrap_or_default();
                self.host = self
                    .config
                    .zitadel_url
                    .clone()
                    .unwrap_or_else(|| self.host.clone());
                self.project = self
                    .config
                    .project_id
                    .clone()
                    .unwrap_or_else(|| self.project.clone());
                self.refresh_runtime().await;
                CanvasMode::Success(MessageState {
                    title: "Config saved".to_string(),
                    lines: vec![format!("Saved to {}", path.display())],
                })
            }
            Err(error) => error_mode("Save config failed", &error.to_string()),
        }
    }
}

fn map_app_record(app: Value) -> Record {
    let kind = app
        .get("oidcConfig")
        .and_then(|oidc| oidc.get("authMethodType"))
        .and_then(|value| value.as_str())
        .map(|value| {
            if value == "OIDC_AUTH_METHOD_TYPE_NONE" {
                "public".to_string()
            } else {
                "confidential".to_string()
            }
        })
        .unwrap_or_else(|| "unknown".to_string());
    let client_id = app
        .get("oidcConfig")
        .and_then(|oidc| oidc.get("clientId"))
        .and_then(|value| value.as_str())
        .unwrap_or("missing-client-id")
        .to_string();
    let redirect_count = app
        .get("oidcConfig")
        .and_then(|oidc| oidc.get("redirectUris"))
        .and_then(|value| value.as_array())
        .map(|uris| uris.len())
        .unwrap_or(0);
    Record {
        id: string_field(&app, "id", "missing-id"),
        name: string_field(&app, "name", "unnamed"),
        kind,
        summary: format!("{redirect_count} redirects"),
        detail: client_id,
        changed_at: string_field(&app, "state", "unknown"),
    }
}

fn map_user_record(user: Value) -> Record {
    Record {
        id: string_field(&user, "id", "missing-id"),
        name: string_field(&user, "userName", "unknown-user"),
        kind: string_field(&user, "state", "unknown"),
        summary: user
            .get("human")
            .and_then(|human| human.get("email"))
            .and_then(|email| email.get("email"))
            .and_then(|email| email.as_str())
            .unwrap_or("no email")
            .to_string(),
        detail: user
            .get("human")
            .and_then(|human| human.get("profile"))
            .and_then(|profile| profile.get("displayName"))
            .and_then(|display_name| display_name.as_str())
            .unwrap_or("human user")
            .to_string(),
        changed_at: "loaded".to_string(),
    }
}

fn map_idp_record(idp: Value) -> Record {
    Record {
        id: string_field(&idp, "id", "missing-id"),
        name: string_field(&idp, "name", "unnamed-idp"),
        kind: string_field(&idp, "state", "unknown"),
        summary: string_field(&idp, "type", "provider"),
        detail: "manual credentials".to_string(),
        changed_at: "configured".to_string(),
    }
}

async fn resolve_project_id(
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

fn error_mode(title: &str, message: &str) -> CanvasMode {
    CanvasMode::Error(MessageState {
        title: title.to_string(),
        lines: vec![message.to_string()],
    })
}

fn text_field(key: &'static str, label: &str, value: &str, help: &str) -> FormField {
    FormField {
        key,
        label: label.to_string(),
        value: value.to_string(),
        kind: crate::tui::FieldKind::Text,
        help: help.to_string(),
    }
}

fn secret_field(key: &'static str, label: &str, value: &str, help: &str) -> FormField {
    FormField {
        key,
        label: label.to_string(),
        value: value.to_string(),
        kind: crate::tui::FieldKind::Secret,
        help: help.to_string(),
    }
}

fn toggle_field(key: &'static str, label: &str, value: bool, help: &str) -> FormField {
    FormField {
        key,
        label: label.to_string(),
        value: if value { "true" } else { "false" }.to_string(),
        kind: crate::tui::FieldKind::Toggle,
        help: help.to_string(),
    }
}

fn choice_field(
    key: &'static str,
    label: &str,
    value: &str,
    options: Vec<String>,
    help: &str,
) -> FormField {
    FormField {
        key,
        label: label.to_string(),
        value: value.to_string(),
        kind: crate::tui::FieldKind::Choice(options),
        help: help.to_string(),
    }
}

fn checkbox_field(key: &'static str, label: &str, value: bool, help: &str) -> FormField {
    FormField {
        key,
        label: label.to_string(),
        value: if value { "true" } else { "false" }.to_string(),
        kind: crate::tui::FieldKind::Checkbox,
        help: help.to_string(),
    }
}

fn form_value(form: &FormState, key: &str) -> String {
    form.fields
        .iter()
        .find(|field| field.key == key)
        .map(|field| field.value.clone())
        .unwrap_or_default()
}

fn optional_value(form: &FormState, key: &str) -> Option<String> {
    let value = form_value(form, key);
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn bool_value(form: &FormState, key: &str) -> bool {
    matches!(form_value(form, key).as_str(), "true" | "1" | "yes" | "on")
}

fn split_csv(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn app_creation_summary(name: &str, value: &Value) -> String {
    format!(
        "{} -> {}",
        name,
        value
            .get("clientId")
            .and_then(|v| v.as_str())
            .unwrap_or("created")
    )
}

async fn create_quick_user(client: &ZitadelClient, user: UserTemplate) -> Result<Vec<String>> {
    let mut lines = Vec::new();
    let result = client
        .create_human_user(&user.email, &user.first_name, &user.last_name, None)
        .await?;
    lines.push(format!("Created {}", user.email));
    if user.admin {
        if let Some(user_id) = result.get("userId").and_then(|v| v.as_str()) {
            client.grant_iam_owner(user_id).await?;
            lines.push(format!("Granted IAM_OWNER to {}", user.email));
        }
    }
    Ok(lines)
}

fn checkbox_enabled(value: &str) -> bool {
    matches!(value, "true" | "1" | "yes" | "on")
}

fn string_field(value: &Value, key: &str, fallback: &str) -> String {
    value
        .get(key)
        .and_then(|field| field.as_str())
        .unwrap_or(fallback)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
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
        conductor.client =
            Some(ZitadelClient::new(server.url(), "test-token".to_string()).unwrap());
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
        conductor.client =
            Some(ZitadelClient::new(server.url(), "test-token".to_string()).unwrap());
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
}
