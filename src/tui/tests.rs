use super::render::{
    cycle_choice, default_setup_form, focus_label, is_enabled, pending_label, render_form_line,
    resource_label, selection_title, status_mark, toggle_field,
};
use super::*;

fn test_app() -> App {
    App::from_bootstrap(TuiBootstrap {
        host: "https://zitadel.example.com".to_string(),
        project: "core".to_string(),
        auth_label: "PAT".to_string(),
        templates_path: Some("/tmp/apps.yml".to_string()),
        setup_required: false,
        app_records: vec![],
        user_records: vec![],
        idp_records: vec![],
    })
}

#[test]
fn focus_cycles_forward() {
    let mut app = test_app();
    app.advance_focus();
    assert_eq!(app.focus, Focus::Actions);
    app.advance_focus();
    assert_eq!(app.focus, Focus::Form);
    app.advance_focus();
    assert_eq!(app.focus, Focus::Records);
    app.advance_focus();
    assert_eq!(app.focus, Focus::Resources);
}

#[test]
fn toggles_inspector_popup() {
    let mut app = test_app();
    assert!(!app.show_inspector);
    app.toggle_inspector();
    assert!(app.show_inspector);
}

#[test]
fn action_navigation_tracks_current_resource() {
    let mut app = test_app();
    app.next_action();
    assert_eq!(
        app.actions()[app.selected_action].label,
        "Regenerate secret"
    );
    app.next_resource();
    assert_eq!(app.actions()[app.selected_action].label, "Create user");
}

#[test]
fn focus_cycles_backward() {
    let mut app = test_app();
    app.reverse_focus();
    assert_eq!(app.focus, Focus::Records);
    app.reverse_focus();
    assert_eq!(app.focus, Focus::Form);
    app.reverse_focus();
    assert_eq!(app.focus, Focus::Actions);
}

#[test]
fn resource_navigation_wraps_and_resets_selection() {
    let mut app = test_app();
    app.selected_action = 2;
    app.selected_record = 1;

    app.next_resource();
    assert_eq!(app.active_resource(), ResourceKind::Users);
    assert_eq!(app.selected_action, 0);
    assert_eq!(app.selected_record, 0);

    app.previous_resource();
    assert_eq!(app.active_resource(), ResourceKind::Applications);
    assert_eq!(app.selected_action, 0);
    assert_eq!(app.selected_record, 0);
}

#[test]
fn empty_bootstrap_keeps_empty_records() {
    let app = App::from_bootstrap(TuiBootstrap {
        host: "https://zitadel.example.com".to_string(),
        project: "core".to_string(),
        auth_label: "PAT".to_string(),
        templates_path: None,
        setup_required: false,
        app_records: vec![],
        user_records: vec![],
        idp_records: vec![],
    });

    assert!(app.app_records.is_empty());
    assert!(app.user_records.is_empty());
    assert!(app.idp_records.is_empty());
    assert_eq!(app.resources[0].count, "0");
    assert_eq!(app.resources[1].count, "0");
    assert_eq!(app.resources[2].count, "0");
    assert!(matches!(app.canvas_mode, CanvasMode::Browse));
    assert!(app.selected_record().is_none());
}

#[test]
fn setup_mode_uses_setup_form() {
    let app = App::from_bootstrap(TuiBootstrap {
        host: "https://zitadel.example.com".to_string(),
        project: "core".to_string(),
        auth_label: "Setup required".to_string(),
        templates_path: None,
        setup_required: true,
        app_records: vec![],
        user_records: vec![],
        idp_records: vec![],
    });

    assert!(matches!(app.canvas_mode, CanvasMode::Setup(_)));
}

#[test]
fn form_editing_changes_selected_field() {
    let mut app = test_app();
    let mut form = default_setup_form(&TuiBootstrap {
        host: "https://zitadel.example.com".to_string(),
        project: "core".to_string(),
        auth_label: "PAT".to_string(),
        templates_path: None,
        setup_required: true,
        app_records: vec![],
        user_records: vec![],
        idp_records: vec![],
    });
    form.selected_field = 3;
    app.set_canvas_mode(CanvasMode::Setup(form));
    app.form_insert_char('x');
    let CanvasMode::Setup(form) = &app.canvas_mode else {
        panic!("expected setup mode");
    };
    assert_eq!(form.fields[3].value, "x");
}

#[test]
fn form_toggle_and_choice_cycle() {
    let mut app = test_app();
    let mut form = default_setup_form(&TuiBootstrap {
        host: "https://zitadel.example.com".to_string(),
        project: "core".to_string(),
        auth_label: "PAT".to_string(),
        templates_path: None,
        setup_required: true,
        app_records: vec![],
        user_records: vec![],
        idp_records: vec![],
    });

    form.selected_field = 2;
    app.set_canvas_mode(CanvasMode::Setup(form));
    app.form_toggle_or_cycle(true);
    let CanvasMode::Setup(form) = &app.canvas_mode else {
        panic!("expected setup mode");
    };
    assert_eq!(form.fields[2].value, "Service account");

    app.form_toggle_or_cycle(true);
    let CanvasMode::Setup(form) = &app.canvas_mode else {
        panic!("expected setup mode");
    };
    assert_eq!(form.fields[2].value, "OAuth device (placeholder)");

    app.form_toggle_or_cycle(false);
    let CanvasMode::Setup(form) = &app.canvas_mode else {
        panic!("expected setup mode");
    };
    assert_eq!(form.fields[2].value, "Service account");
}

#[test]
fn reset_to_browse_returns_to_setup_when_required() {
    let mut app = App::from_bootstrap(TuiBootstrap {
        host: "https://zitadel.example.com".to_string(),
        project: "core".to_string(),
        auth_label: "Setup required".to_string(),
        templates_path: Some("/tmp/apps.yml".to_string()),
        setup_required: true,
        app_records: vec![],
        user_records: vec![],
        idp_records: vec![],
    });

    app.reset_to_browse();
    assert!(matches!(app.canvas_mode, CanvasMode::Setup(_)));
    assert_eq!(app.focus, Focus::Resources);
}

#[test]
fn reset_to_browse_returns_to_browser_when_ready() {
    let mut app = test_app();
    app.reset_to_browse();
    assert!(matches!(app.canvas_mode, CanvasMode::Browse));
    assert_eq!(app.focus, Focus::Resources);
}

#[test]
fn sync_runtime_updates_counts_without_fallback_records() {
    let mut app = test_app();
    app.selected_record = 1;
    app.selected_action = 2;

    app.sync_runtime(TuiBootstrap {
        host: "https://zitadel.example.com".to_string(),
        project: "ops".to_string(),
        auth_label: "Service account".to_string(),
        templates_path: None,
        setup_required: true,
        app_records: vec![],
        user_records: vec![],
        idp_records: vec![],
    });

    assert_eq!(app.project, "ops");
    assert_eq!(app.auth_label, "Service account");
    assert_eq!(app.resources[0].count, "0");
    assert_eq!(app.resources[1].count, "0");
    assert_eq!(app.resources[2].count, "0");
    assert_eq!(app.resources[3].count, "setup");
    assert!(app.app_records.is_empty());
    assert!(app.user_records.is_empty());
    assert!(app.idp_records.is_empty());
    assert_eq!(app.selected_record, 0);
    assert_eq!(app.selected_action, 0);
}

#[test]
fn record_navigation_wraps_even_when_empty() {
    let mut app = test_app();
    app.next_record();
    assert_eq!(app.focus, Focus::Records);
    assert_eq!(app.selected_record, 0);
    app.previous_record();
    assert_eq!(app.focus, Focus::Records);
    assert_eq!(app.selected_record, 0);
}

#[test]
fn render_form_line_text_field_selected() {
    let field = FormField {
        key: "host",
        label: "Host".to_string(),
        value: "https://z.example.com".to_string(),
        kind: FieldKind::Text,
        help: String::new(),
    };
    let line = render_form_line(&field, true);
    assert!(line.starts_with("›"));
    assert!(line.contains("Host"));
    assert!(line.contains("https://z.example.com"));
}

#[test]
fn render_form_line_text_field_unselected() {
    let field = FormField {
        key: "host",
        label: "Host".to_string(),
        value: "value".to_string(),
        kind: FieldKind::Text,
        help: String::new(),
    };
    let line = render_form_line(&field, false);
    assert!(line.starts_with(" "));
}

#[test]
fn render_form_line_secret_masks_value() {
    let field = FormField {
        key: "token",
        label: "PAT".to_string(),
        value: "abc".to_string(),
        kind: FieldKind::Secret,
        help: String::new(),
    };
    let line = render_form_line(&field, false);
    assert!(line.contains("•••"));
    assert!(!line.contains("abc"));
}

#[test]
fn render_form_line_secret_empty_shows_single_dot() {
    let field = FormField {
        key: "token",
        label: "PAT".to_string(),
        value: String::new(),
        kind: FieldKind::Secret,
        help: String::new(),
    };
    let line = render_form_line(&field, false);
    assert!(line.contains("•"));
}

#[test]
fn render_form_line_toggle_enabled() {
    let field = FormField {
        key: "flag",
        label: "Admin".to_string(),
        value: "true".to_string(),
        kind: FieldKind::Toggle,
        help: String::new(),
    };
    let line = render_form_line(&field, false);
    assert!(line.contains("[x]"));
}

#[test]
fn render_form_line_toggle_disabled() {
    let field = FormField {
        key: "flag",
        label: "Admin".to_string(),
        value: "false".to_string(),
        kind: FieldKind::Toggle,
        help: String::new(),
    };
    let line = render_form_line(&field, false);
    assert!(line.contains("[ ]"));
}

#[test]
fn render_form_line_checkbox_enabled() {
    let field = FormField {
        key: "cb",
        label: "Enable".to_string(),
        value: "true".to_string(),
        kind: FieldKind::Checkbox,
        help: String::new(),
    };
    let line = render_form_line(&field, false);
    assert!(line.contains("[x]"));
}

#[test]
fn render_form_line_choice_shows_value() {
    let field = FormField {
        key: "method",
        label: "Auth".to_string(),
        value: "PAT".to_string(),
        kind: FieldKind::Choice(vec!["PAT".to_string(), "SA".to_string()]),
        help: String::new(),
    };
    let line = render_form_line(&field, false);
    assert!(line.contains("PAT"));
}

#[test]
fn pending_label_covers_all_variants() {
    assert_eq!(
        pending_label(&PendingAction::CreateApplication),
        "create application"
    );
    assert_eq!(
        pending_label(&PendingAction::QuickSetupApplications),
        "quick setup apps"
    );
    assert_eq!(
        pending_label(&PendingAction::DeleteApplication {
            app_id: "a".to_string(),
            name: "b".to_string()
        }),
        "delete application"
    );
    assert_eq!(
        pending_label(&PendingAction::RegenerateSecret {
            app_id: "a".to_string(),
            name: "b".to_string(),
            client_id: "c".to_string()
        }),
        "regenerate secret"
    );
    assert_eq!(pending_label(&PendingAction::CreateUser), "create user");
    assert_eq!(
        pending_label(&PendingAction::CreateAdminUser),
        "create admin user"
    );
    assert_eq!(
        pending_label(&PendingAction::GrantIamOwner {
            user_id: "u".to_string(),
            username: "n".to_string()
        }),
        "grant IAM_OWNER"
    );
    assert_eq!(
        pending_label(&PendingAction::QuickSetupUsers),
        "quick setup users"
    );
    assert_eq!(
        pending_label(&PendingAction::ConfigureGoogleIdp),
        "configure Google IDP"
    );
    assert_eq!(
        pending_label(&PendingAction::ValidateAuthSetup),
        "validate auth setup"
    );
    assert_eq!(pending_label(&PendingAction::SaveConfig), "save config");
}

#[test]
fn is_enabled_recognizes_truthy_values() {
    assert!(is_enabled("true"));
    assert!(is_enabled("yes"));
    assert!(is_enabled("on"));
    assert!(is_enabled("1"));
    assert!(!is_enabled("false"));
    assert!(!is_enabled(""));
    assert!(!is_enabled("maybe"));
}

#[test]
fn toggle_field_flips_value() {
    let mut field = FormField {
        key: "flag",
        label: "Flag".to_string(),
        value: "false".to_string(),
        kind: FieldKind::Toggle,
        help: String::new(),
    };
    toggle_field(&mut field);
    assert_eq!(field.value, "true");
    toggle_field(&mut field);
    assert_eq!(field.value, "false");
}

#[test]
fn cycle_choice_forward() {
    let options = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let mut field = FormField {
        key: "opt",
        label: "Opt".to_string(),
        value: "a".to_string(),
        kind: FieldKind::Choice(options.clone()),
        help: String::new(),
    };
    cycle_choice(&mut field, &options, true);
    assert_eq!(field.value, "b");
    cycle_choice(&mut field, &options, true);
    assert_eq!(field.value, "c");
    cycle_choice(&mut field, &options, true);
    assert_eq!(field.value, "a");
}

#[test]
fn cycle_choice_backward() {
    let options = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let mut field = FormField {
        key: "opt",
        label: "Opt".to_string(),
        value: "a".to_string(),
        kind: FieldKind::Choice(options.clone()),
        help: String::new(),
    };
    cycle_choice(&mut field, &options, false);
    assert_eq!(field.value, "c");
    cycle_choice(&mut field, &options, false);
    assert_eq!(field.value, "b");
}

#[test]
fn cycle_choice_unknown_value_resets_to_first() {
    let options = vec!["a".to_string(), "b".to_string()];
    let mut field = FormField {
        key: "opt",
        label: "Opt".to_string(),
        value: "unknown".to_string(),
        kind: FieldKind::Choice(options.clone()),
        help: String::new(),
    };
    cycle_choice(&mut field, &options, true);
    assert_eq!(field.value, "a");
}

#[test]
fn form_backspace_removes_last_char() {
    let mut app = test_app();
    let form = FormState {
        title: "Test".to_string(),
        description: String::new(),
        submit_label: String::new(),
        fields: vec![FormField {
            key: "name",
            label: "Name".to_string(),
            value: "hello".to_string(),
            kind: FieldKind::Text,
            help: String::new(),
        }],
        selected_field: 0,
        pending: PendingAction::SaveConfig,
    };
    app.set_canvas_mode(CanvasMode::EditForm(form));
    app.form_backspace();
    let CanvasMode::EditForm(form) = &app.canvas_mode else {
        panic!("expected EditForm");
    };
    assert_eq!(form.fields[0].value, "hell");
}

#[test]
fn form_backspace_noop_on_toggle() {
    let mut app = test_app();
    let form = FormState {
        title: "Test".to_string(),
        description: String::new(),
        submit_label: String::new(),
        fields: vec![FormField {
            key: "flag",
            label: "Flag".to_string(),
            value: "true".to_string(),
            kind: FieldKind::Toggle,
            help: String::new(),
        }],
        selected_field: 0,
        pending: PendingAction::SaveConfig,
    };
    app.set_canvas_mode(CanvasMode::EditForm(form));
    app.form_backspace();
    let CanvasMode::EditForm(form) = &app.canvas_mode else {
        panic!("expected EditForm");
    };
    assert_eq!(form.fields[0].value, "true");
}

#[test]
fn form_next_field_wraps() {
    let mut app = test_app();
    let form = FormState {
        title: "Test".to_string(),
        description: String::new(),
        submit_label: String::new(),
        fields: vec![
            FormField {
                key: "a",
                label: "A".to_string(),
                value: String::new(),
                kind: FieldKind::Text,
                help: String::new(),
            },
            FormField {
                key: "b",
                label: "B".to_string(),
                value: String::new(),
                kind: FieldKind::Text,
                help: String::new(),
            },
        ],
        selected_field: 0,
        pending: PendingAction::SaveConfig,
    };
    app.set_canvas_mode(CanvasMode::EditForm(form));
    app.form_next_field();
    let CanvasMode::EditForm(form) = &app.canvas_mode else {
        panic!("expected EditForm");
    };
    assert_eq!(form.selected_field, 1);
    app.form_next_field();
    let CanvasMode::EditForm(form) = &app.canvas_mode else {
        panic!("expected EditForm");
    };
    assert_eq!(form.selected_field, 0);
}

#[test]
fn form_previous_field_wraps() {
    let mut app = test_app();
    let form = FormState {
        title: "Test".to_string(),
        description: String::new(),
        submit_label: String::new(),
        fields: vec![
            FormField {
                key: "a",
                label: "A".to_string(),
                value: String::new(),
                kind: FieldKind::Text,
                help: String::new(),
            },
            FormField {
                key: "b",
                label: "B".to_string(),
                value: String::new(),
                kind: FieldKind::Text,
                help: String::new(),
            },
        ],
        selected_field: 0,
        pending: PendingAction::SaveConfig,
    };
    app.set_canvas_mode(CanvasMode::EditForm(form));
    app.form_previous_field();
    let CanvasMode::EditForm(form) = &app.canvas_mode else {
        panic!("expected EditForm");
    };
    assert_eq!(form.selected_field, 1);
}

#[test]
fn selected_record_returns_record_when_present() {
    let mut app = App::from_bootstrap(TuiBootstrap {
        host: "https://zitadel.example.com".to_string(),
        project: "core".to_string(),
        auth_label: "PAT".to_string(),
        templates_path: None,
        setup_required: false,
        app_records: vec![Record {
            id: "app-1".to_string(),
            name: "grafana".to_string(),
            kind: "public".to_string(),
            summary: "1 redirect".to_string(),
            detail: "cid-1".to_string(),
            changed_at: "ACTIVE".to_string(),
        }],
        user_records: vec![],
        idp_records: vec![],
    });
    app.selected_record = 0;
    let record = app.selected_record();
    assert!(record.is_some());
    assert_eq!(record.unwrap().name, "grafana");
}

#[test]
fn resource_label_covers_all_kinds() {
    assert_eq!(resource_label(ResourceKind::Applications), "Applications");
    assert_eq!(resource_label(ResourceKind::Users), "Users");
    assert_eq!(resource_label(ResourceKind::Idps), "IDPs");
    assert_eq!(resource_label(ResourceKind::Auth), "Auth");
    assert_eq!(resource_label(ResourceKind::Config), "Config");
}

#[test]
fn focus_label_covers_all_foci() {
    assert_eq!(focus_label(Focus::Resources), "resources");
    assert_eq!(focus_label(Focus::Actions), "actions");
    assert_eq!(focus_label(Focus::Form), "form");
    assert_eq!(focus_label(Focus::Records), "records");
}

#[test]
fn status_mark_setup_required() {
    let mut app = test_app();
    app.setup_required = true;
    assert_eq!(status_mark(&app), "!");
}

#[test]
fn status_mark_ready() {
    let app = test_app();
    assert_eq!(status_mark(&app), "✓");
}

#[test]
fn selection_title_per_resource() {
    let mut app = test_app();
    assert_eq!(selection_title(&app), "existing applications");
    app.next_resource();
    assert_eq!(selection_title(&app), "existing users");
    app.next_resource();
    assert_eq!(selection_title(&app), "configured identity providers");
}

#[test]
fn set_canvas_mode_sets_focus_to_form_for_edit() {
    let mut app = test_app();
    let form = FormState {
        title: "Test".to_string(),
        description: String::new(),
        submit_label: String::new(),
        fields: vec![FormField {
            key: "a",
            label: "A".to_string(),
            value: String::new(),
            kind: FieldKind::Text,
            help: String::new(),
        }],
        selected_field: 0,
        pending: PendingAction::SaveConfig,
    };
    app.set_canvas_mode(CanvasMode::EditForm(form));
    assert_eq!(app.focus, Focus::Form);
}

#[test]
fn set_canvas_mode_browse_sets_focus_to_resources() {
    let mut app = test_app();
    app.focus = Focus::Form;
    app.set_canvas_mode(CanvasMode::Browse);
    assert_eq!(app.focus, Focus::Resources);
}

#[test]
fn canvas_title_browse_mode() {
    let app = test_app();
    let title = app.canvas_title();
    assert!(!title.is_empty());
}

#[test]
fn canvas_title_error_mode() {
    let mut app = test_app();
    app.set_canvas_mode(CanvasMode::Error(MessageState {
        title: "Something failed".to_string(),
        lines: vec!["detail".to_string()],
    }));
    assert_eq!(app.canvas_title(), "Something failed");
}

#[test]
fn message_lines_success_mode() {
    let mut app = test_app();
    app.set_canvas_mode(CanvasMode::Success(MessageState {
        title: "Done".to_string(),
        lines: vec!["line1".to_string(), "line2".to_string()],
    }));
    let lines = app.message_lines();
    assert_eq!(lines, vec!["line1".to_string(), "line2".to_string()]);
}

#[test]
fn message_lines_form_mode_renders_fields() {
    let mut app = test_app();
    let form = FormState {
        title: "Test".to_string(),
        description: String::new(),
        submit_label: String::new(),
        fields: vec![FormField {
            key: "host",
            label: "Host".to_string(),
            value: "https://z.example.com".to_string(),
            kind: FieldKind::Text,
            help: String::new(),
        }],
        selected_field: 0,
        pending: PendingAction::SaveConfig,
    };
    app.set_canvas_mode(CanvasMode::EditForm(form));
    let lines = app.message_lines();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("Host"));
}

#[test]
fn record_navigation_wraps_with_records() {
    let mut app = App::from_bootstrap(TuiBootstrap {
        host: "https://zitadel.example.com".to_string(),
        project: "core".to_string(),
        auth_label: "PAT".to_string(),
        templates_path: None,
        setup_required: false,
        app_records: vec![
            Record {
                id: "a1".to_string(),
                name: "app1".to_string(),
                kind: "public".to_string(),
                summary: String::new(),
                detail: String::new(),
                changed_at: String::new(),
            },
            Record {
                id: "a2".to_string(),
                name: "app2".to_string(),
                kind: "public".to_string(),
                summary: String::new(),
                detail: String::new(),
                changed_at: String::new(),
            },
        ],
        user_records: vec![],
        idp_records: vec![],
    });
    app.next_record();
    assert_eq!(app.selected_record, 1);
    app.next_record();
    assert_eq!(app.selected_record, 0);
    app.previous_record();
    assert_eq!(app.selected_record, 1);
}

#[test]
fn form_insert_char_space_toggles_toggle_field() {
    let mut app = test_app();
    let form = FormState {
        title: "Test".to_string(),
        description: String::new(),
        submit_label: String::new(),
        fields: vec![FormField {
            key: "flag",
            label: "Admin".to_string(),
            value: "false".to_string(),
            kind: FieldKind::Toggle,
            help: String::new(),
        }],
        selected_field: 0,
        pending: PendingAction::SaveConfig,
    };
    app.set_canvas_mode(CanvasMode::EditForm(form));
    app.form_insert_char(' ');
    let CanvasMode::EditForm(form) = &app.canvas_mode else {
        panic!("expected EditForm");
    };
    assert_eq!(form.fields[0].value, "true");
}

#[test]
fn form_insert_char_space_cycles_choice() {
    let mut app = test_app();
    let form = FormState {
        title: "Test".to_string(),
        description: String::new(),
        submit_label: String::new(),
        fields: vec![FormField {
            key: "method",
            label: "Auth".to_string(),
            value: "PAT".to_string(),
            kind: FieldKind::Choice(vec!["PAT".to_string(), "SA".to_string()]),
            help: String::new(),
        }],
        selected_field: 0,
        pending: PendingAction::SaveConfig,
    };
    app.set_canvas_mode(CanvasMode::EditForm(form));
    app.form_insert_char(' ');
    let CanvasMode::EditForm(form) = &app.canvas_mode else {
        panic!("expected EditForm");
    };
    assert_eq!(form.fields[0].value, "SA");
}

#[test]
fn action_navigation_wraps() {
    let mut app = test_app();
    let action_count = app.actions().len();
    for _ in 0..action_count {
        app.next_action();
    }
    assert_eq!(app.selected_action, 0);
}

#[test]
fn previous_action_wraps_to_last() {
    let mut app = test_app();
    app.previous_action();
    let last = app.actions().len() - 1;
    assert_eq!(app.selected_action, last);
}
