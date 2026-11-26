use anyhow::{ensure, Context, Result};

use super::{
    forms::{choice_field, form_value},
    support::error_mode,
    TuiConductor,
};
use crate::tui::{CanvasMode, ConfirmState, FormState, MessageState, PendingAction, Record};

impl TuiConductor {
    pub async fn begin_user_action(&self, index: usize, record: Option<&Record>) -> CanvasMode {
        if self.cli.demo {
            return match (index, record) {
                (4, Some(user)) => CanvasMode::Success(MessageState {
                    title: "Project roles".into(),
                    lines: vec![format!("{} · {}", user.name, self.project), "reader".into()],
                }),
                _ => Self::demo_preview(),
            };
        }
        match self.user_action(index, record).await {
            Ok(mode) => mode,
            Err(error) => error_mode("User action unavailable", &error.to_string()),
        }
    }

    async fn user_action(&self, index: usize, record: Option<&Record>) -> Result<CanvasMode> {
        let user = record.context("Select a user first.")?;
        ensure!(
            !user.id.is_empty() && user.id != "missing-id",
            "Selected user has no ID."
        );
        let client = self
            .client
            .as_ref()
            .context("Run authentication setup first.")?;
        if matches!(index, 7 | 8) {
            let unlock = index == 7;
            let (title, impact) = if unlock {
                ("Unlock user", "The user will be able to sign in again.")
            } else {
                (
                    "Deactivate user",
                    "The user will lose access across projects.",
                )
            };
            return Ok(review(
                title,
                vec![format!("{} ({})", user.name, user.id), impact.to_owned()],
                PendingAction::UserLifecycle {
                    user: Box::new(user.clone()),
                    unlock,
                },
            ));
        }
        ensure!(
            !self.project.is_empty() && self.project != "default",
            "Select a project first."
        );
        let grant = client.user_project_grant(&user.id, &self.project).await?;
        let current = grant
            .as_ref()
            .map(|g| g.role_keys.clone())
            .unwrap_or_default();
        let subject = format!("{} ({}) · project {}", user.name, user.id, self.project);
        if index == 4 {
            let mut lines = vec![subject];
            if current.is_empty() {
                lines.push("No roles assigned.".into());
            } else {
                lines.extend(current);
            }
            return Ok(CanvasMode::Success(MessageState {
                title: "Project roles".into(),
                lines,
            }));
        }
        ensure!(matches!(index, 5 | 6), "Unknown role action.");
        let add = index == 5;
        let choices = if add {
            client
                .project_role_keys(&self.project)
                .await?
                .into_iter()
                .filter(|role| !current.contains(role))
                .collect()
        } else {
            current
        };
        let Some(first) = choices.first().cloned() else {
            return Ok(CanvasMode::Success(MessageState {
                title: "Project roles".into(),
                lines: vec![
                    subject,
                    if add {
                        "No more roles available to grant."
                    } else {
                        "No roles available to revoke."
                    }
                    .into(),
                ],
            }));
        };
        Ok(CanvasMode::EditForm(FormState {
            title: if add {
                "Grant project role"
            } else {
                "Revoke project role"
            }
            .into(),
            description: subject,
            submit_label: "[Enter] review".into(),
            selected_field: 0,
            fields: vec![choice_field(
                "role",
                "Role",
                &first,
                choices,
                "Choose a project role",
            )],
            pending: PendingAction::UserRole {
                user: Box::new(user.clone()),
                project: self.project.clone(),
                grant,
                add,
                role: None,
            },
        }))
    }

    pub(super) fn review_user_role(form: &FormState) -> CanvasMode {
        let mut pending = form.pending.clone();
        let PendingAction::UserRole {
            role,
            user,
            project,
            add,
            ..
        } = &mut pending
        else {
            return error_mode("Invalid action", "Select a project role.");
        };
        let selected = form_value(form, "role");
        if selected.is_empty() {
            return error_mode("Invalid role", "Choose a project role.");
        }
        *role = Some(selected.clone());
        let lines = vec![
            format!("{} ({})", user.name, user.id),
            format!("Project: {project}"),
            format!("Role: {selected}"),
        ];
        let title = if *add {
            "Grant project role"
        } else {
            "Revoke project role"
        };
        review(title, lines, pending)
    }

    pub(super) async fn confirm_user_access(&mut self, pending: PendingAction) -> CanvasMode {
        let Some(client) = self.client.as_ref() else {
            return error_mode("Authentication required", "Run setup first.");
        };
        let (result, message) = match pending {
            PendingAction::UserRole {
                user,
                project,
                grant,
                add,
                role: Some(role),
            } if project == self.project => {
                let result = client
                    .change_user_role(&user.id, &project, grant.as_ref(), &role, add)
                    .await;
                let verb = if add { "Granted" } else { "Revoked" };
                (
                    result,
                    format!("{verb} {role} · {} · project {project}", user.name),
                )
            }
            PendingAction::UserLifecycle { user, unlock } => {
                let result = client.change_user_state(&user.id, unlock).await;
                let verb = if unlock { "Unlocked" } else { "Deactivated" };
                (result, format!("{verb} {}", user.name))
            }
            _ => {
                return error_mode(
                    "Invalid confirmation",
                    "Review this action in the current project.",
                )
            }
        };
        self.finish_simple_mutation(result, "User access updated", vec![message])
            .await
    }
}

fn review(title: &str, lines: Vec<String>, pending: PendingAction) -> CanvasMode {
    CanvasMode::Confirm(ConfirmState {
        title: title.into(),
        lines,
        submit_label: "[Enter] confirm".into(),
        pending,
    })
}
