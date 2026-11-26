use super::{support::error_mode, TuiConductor};
use crate::tui::{CanvasMode, MessageState, PendingAction, Record};

impl TuiConductor {
    pub(super) fn refresh_demo(&mut self) {
        self.clear_directory();
        self.host = "Demo mode".into();
        self.auth_label = "Demo data".into();
        self.setup_required = false;
        self.organizations = choices(&[("northwind", "Northwind Labs"), ("cedar", "Cedar Studio")]);
        let organization = self.organization.get_or_insert_with(|| "northwind".into());
        let (projects, people) = if organization == "northwind" {
            (
                &[
                    ("platform", "Developer Platform"),
                    ("support", "Customer Support"),
                ][..],
                &[
                    ("maya", "Maya Chen", "ACTIVE"),
                    ("samir", "Samir Patel", "LOCKED"),
                    ("avery", "Avery Brooks", "ACTIVE"),
                ][..],
            )
        } else {
            (
                &[("studio", "Studio Tools")][..],
                &[
                    ("alex", "Alex Rivera", "ACTIVE"),
                    ("jules", "Jules Park", "ACTIVE"),
                ][..],
            )
        };
        self.projects = choices(projects);
        if !self.projects.iter().any(|(id, _)| *id == self.project) {
            self.project.clone_from(&self.projects[0].0);
        }
        if self.demo_apps.is_empty() {
            self.demo_apps = vec![
                (
                    "platform",
                    application(
                        "atlas",
                        "Atlas Console",
                        "confidential",
                        &["https://atlas.example.test/callback"],
                    ),
                ),
                (
                    "platform",
                    application("desktop", "Desktop Client", "native", &["atlas://signin"]),
                ),
                (
                    "platform",
                    application("reports", "Reporting API", "api", &[]),
                ),
                (
                    "support",
                    application(
                        "portal",
                        "Customer Portal",
                        "public",
                        &["https://portal.example.test/callback"],
                    ),
                ),
                (
                    "studio",
                    application(
                        "review",
                        "Review Dashboard",
                        "confidential",
                        &["https://review.example.test/callback"],
                    ),
                ),
            ];
        }
        self.app_records = self
            .demo_apps
            .iter()
            .filter(|(project, _)| *project == self.project)
            .map(|(_, record)| record.clone())
            .collect();
        self.user_records = people
            .iter()
            .map(|(id, name, state)| Record {
                id: (*id).into(),
                name: (*name).into(),
                kind: format!("human USER_STATE_{state}"),
                summary: format!("{id}@{organization}.example.test"),
                ..Record::default()
            })
            .collect();
        self.idp_records = vec![Record {
            id: "demo-google".into(),
            name: "Google Workspace".into(),
            kind: "Active".into(),
            summary: "PROVIDER_TYPE_GOOGLE".into(),
            detail: "Instance · Enabled for sign-in".into(),
            provider_owner: Some("Instance"),
            ..Record::default()
        }];
    }

    pub(super) fn demo_preview() -> CanvasMode {
        error_mode(
            "Demo data",
            "This action is a preview. Edit an application's redirect URIs to try an in-memory change.",
        )
    }

    pub(super) fn confirm_demo(&mut self, pending: PendingAction) -> CanvasMode {
        let PendingAction::ApplyRedirectUris {
            app_id,
            original,
            desired,
        } = pending
        else {
            return Self::demo_preview();
        };
        let Some((_, record)) = self.demo_apps.iter_mut().find(|(project, record)| {
            *project == self.project && record.id == app_id && record.redirect_uris == original
        }) else {
            return error_mode(
                "Application changed",
                "Refresh and open the redirect editor again.",
            );
        };
        record.redirect_uris.clone_from(&desired);
        record.summary = redirect_summary(desired.len());
        self.refresh_demo();
        CanvasMode::Success(MessageState {
            title: "Demo redirect URIs updated".into(),
            lines: desired,
        })
    }
}

fn choices(choices: &[(&str, &str)]) -> Vec<(String, String)> {
    choices
        .iter()
        .map(|(id, name)| ((*id).into(), (*name).into()))
        .collect()
}

fn redirect_summary(count: usize) -> String {
    if count == 1 {
        "1 redirect".into()
    } else {
        format!("{count} redirects")
    }
}

fn application(id: &str, name: &str, kind: &str, redirects: &[&str]) -> Record {
    Record {
        id: id.into(),
        name: name.into(),
        kind: kind.into(),
        summary: if kind == "api" {
            "Private key JWT".into()
        } else {
            redirect_summary(redirects.len())
        },
        detail: format!("{id}-demo-client"),
        redirect_uris: redirects.iter().map(|uri| (*uri).into()).collect(),
        ..Record::default()
    }
}
