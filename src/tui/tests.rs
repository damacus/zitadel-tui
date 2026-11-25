use super::{
    render::{canvas_lines, canvas_title},
    state::{cycle_choice, is_enabled, toggle_field},
    types::{
        default_setup_form, App, AppCommand, CanvasMode, FieldKind, Focus, FormField, FormState,
        MessageState, PendingAction, Record, ResourceKind, TuiBootstrap,
    },
    widgets::render_form_line,
};

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
        idp_error: None,
    })
}

#[test]
fn enter_opens_section_then_runs_selected_action() {
    let mut app = test_app();
    app.handle_key(crossterm::event::KeyCode::Enter);
    assert_eq!(
        app.handle_key(crossterm::event::KeyCode::Enter),
        AppCommand::BeginAction {
            resource: ResourceKind::Applications,
            action_index: 0,
            selected_record: None,
        }
    );
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
        idp_error: None,
    });

    assert!(app.app_records.is_empty());
    assert!(app.user_records.is_empty());
    assert!(app.idp_records.is_empty());
    assert!(matches!(app.canvas_mode, CanvasMode::Browse));
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
        idp_error: None,
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
        idp_error: None,
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
        idp_error: None,
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
        idp_error: None,
    });

    app.reset_to_browse();
    assert!(matches!(app.canvas_mode, CanvasMode::Setup(_)));
    assert_eq!(app.focus, Focus::Form);
}

#[test]
fn reset_to_browse_returns_to_browser_when_ready() {
    let mut app = test_app();
    app.reset_to_browse();
    assert!(matches!(app.canvas_mode, CanvasMode::Browse));
    assert_eq!(app.focus, Focus::Records);
}

#[test]
fn sync_runtime_updates_counts_without_fallback_records() {
    let mut app = test_app();

    app.sync_runtime(TuiBootstrap {
        host: "https://zitadel.example.com".to_string(),
        project: "ops".to_string(),
        auth_label: "Service account".to_string(),
        templates_path: None,
        setup_required: true,
        app_records: vec![],
        user_records: vec![],
        idp_records: vec![],
        idp_error: None,
    });

    assert_eq!(app.project, "ops");
    assert_eq!(app.auth_label, "Service account");
    assert!(app.app_records.is_empty());
    assert!(app.user_records.is_empty());
    assert!(app.idp_records.is_empty());
}

#[test]
fn render_form_line_text_field_selected() {
    let field = FormField {
        key: "host",
        label: "Host".to_string(),
        value: "https://z.example.com".to_string(),
        kind: FieldKind::Text,
        help: String::new(),
        cursor: 0,
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
        cursor: 0,
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
        cursor: 0,
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
        cursor: 0,
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
        cursor: 0,
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
        cursor: 0,
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
        cursor: 0,
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
        cursor: 0,
    };
    let line = render_form_line(&field, false);
    assert!(line.contains("PAT"));
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
        cursor: 0,
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
        cursor: 0,
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
        cursor: 0,
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
        cursor: 0,
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
            cursor: 5,
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
            cursor: 0,
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
                cursor: 0,
            },
            FormField {
                key: "b",
                label: "B".to_string(),
                value: String::new(),
                kind: FieldKind::Text,
                help: String::new(),
                cursor: 0,
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
                cursor: 0,
            },
            FormField {
                key: "b",
                label: "B".to_string(),
                value: String::new(),
                kind: FieldKind::Text,
                help: String::new(),
                cursor: 0,
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
            cursor: 0,
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
    let title = canvas_title(&app);
    assert!(!title.is_empty());
}

#[test]
fn canvas_title_error_mode() {
    let mut app = test_app();
    app.set_canvas_mode(CanvasMode::Error(MessageState {
        title: "Something failed".to_string(),
        lines: vec!["detail".to_string()],
    }));
    assert_eq!(canvas_title(&app), "Something failed");
}

#[test]
fn message_lines_success_mode() {
    let mut app = test_app();
    app.set_canvas_mode(CanvasMode::Success(MessageState {
        title: "Done".to_string(),
        lines: vec!["line1".to_string(), "line2".to_string()],
    }));
    let lines = canvas_lines(&app);
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
            cursor: 0,
        }],
        selected_field: 0,
        pending: PendingAction::SaveConfig,
    };
    app.set_canvas_mode(CanvasMode::EditForm(form));
    let lines = canvas_lines(&app);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("Host"));
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
            cursor: 0,
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
            cursor: 0,
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
fn typing_n_in_form_field_inserts_text_not_navigates() {
    let mut app = test_app();
    let form = FormState {
        title: "Create application".to_string(),
        description: String::new(),
        submit_label: String::new(),
        fields: vec![FormField {
            key: "name",
            label: "Name".to_string(),
            value: String::new(),
            kind: FieldKind::Text,
            help: String::new(),
            cursor: 0,
        }],
        selected_field: 0,
        pending: PendingAction::CreateApplication,
    };
    app.set_canvas_mode(CanvasMode::EditForm(form));
    assert_eq!(app.focus, Focus::Form);

    app.handle_key(crossterm::event::KeyCode::Char('n'));
    let CanvasMode::EditForm(form) = &app.canvas_mode else {
        panic!("expected EditForm");
    };
    assert_eq!(form.fields[0].value, "n");
    assert_eq!(app.focus, Focus::Form);
}

#[test]
fn typing_j_in_form_field_inserts_text_not_moves_field() {
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
                cursor: 0,
            },
            FormField {
                key: "b",
                label: "B".to_string(),
                value: String::new(),
                kind: FieldKind::Text,
                help: String::new(),
                cursor: 0,
            },
        ],
        selected_field: 0,
        pending: PendingAction::SaveConfig,
    };
    app.set_canvas_mode(CanvasMode::EditForm(form));
    app.handle_key(crossterm::event::KeyCode::Char('j'));
    let CanvasMode::EditForm(form) = &app.canvas_mode else {
        panic!("expected EditForm");
    };
    assert_eq!(form.fields[0].value, "j");
    assert_eq!(form.selected_field, 0);
}

#[test]
fn typing_g_in_form_field_inserts_text_not_focuses_resources() {
    let mut app = test_app();
    let form = FormState {
        title: "Test".to_string(),
        description: String::new(),
        submit_label: String::new(),
        fields: vec![FormField {
            key: "host",
            label: "Host".to_string(),
            value: String::new(),
            kind: FieldKind::Text,
            help: String::new(),
            cursor: 0,
        }],
        selected_field: 0,
        pending: PendingAction::SaveConfig,
    };
    app.set_canvas_mode(CanvasMode::EditForm(form));
    app.handle_key(crossterm::event::KeyCode::Char('g'));
    let CanvasMode::EditForm(form) = &app.canvas_mode else {
        panic!("expected EditForm");
    };
    assert_eq!(form.fields[0].value, "g");
    assert_eq!(app.focus, Focus::Form);
}

#[test]
fn typing_q_in_form_field_inserts_text_not_quits() {
    let mut app = test_app();
    let form = FormState {
        title: "Test".to_string(),
        description: String::new(),
        submit_label: String::new(),
        fields: vec![FormField {
            key: "name",
            label: "Name".to_string(),
            value: String::new(),
            kind: FieldKind::Text,
            help: String::new(),
            cursor: 0,
        }],
        selected_field: 0,
        pending: PendingAction::SaveConfig,
    };
    app.set_canvas_mode(CanvasMode::EditForm(form));
    let cmd = app.handle_key(crossterm::event::KeyCode::Char('q'));
    let CanvasMode::EditForm(form) = &app.canvas_mode else {
        panic!("expected EditForm");
    };
    assert_eq!(form.fields[0].value, "q");
    assert!(matches!(cmd, AppCommand::Noop));
}

#[test]
fn form_insert_char_at_cursor_position() {
    let mut app = test_app();
    let form = FormState {
        title: "Test".to_string(),
        description: String::new(),
        submit_label: String::new(),
        fields: vec![FormField {
            key: "name",
            label: "Name".to_string(),
            value: "hel".to_string(),
            kind: FieldKind::Text,
            help: String::new(),
            cursor: 3,
        }],
        selected_field: 0,
        pending: PendingAction::SaveConfig,
    };
    app.set_canvas_mode(CanvasMode::EditForm(form));
    app.form_insert_char('l');
    app.form_insert_char('o');
    let CanvasMode::EditForm(form) = &app.canvas_mode else {
        panic!("expected EditForm");
    };
    assert_eq!(form.fields[0].value, "hello");
}

#[test]
fn form_backspace_at_cursor_removes_char_before() {
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
            cursor: 5,
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
fn form_cursor_moves_left_and_right_within_value() {
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
            cursor: 5,
        }],
        selected_field: 0,
        pending: PendingAction::SaveConfig,
    };
    app.set_canvas_mode(CanvasMode::EditForm(form));
    app.form_cursor_left();
    app.form_insert_char('x');
    let CanvasMode::EditForm(form) = &app.canvas_mode else {
        panic!("expected EditForm");
    };
    assert_eq!(form.fields[0].value, "hellxo");

    app.form_cursor_left();
    app.form_cursor_left();
    app.form_insert_char('y');
    let CanvasMode::EditForm(form) = &app.canvas_mode else {
        panic!("expected EditForm");
    };
    assert_eq!(form.fields[0].value, "helylxo");
}

#[test]
fn form_cursor_clamped_at_value_boundaries() {
    let mut app = test_app();
    let form = FormState {
        title: "Test".to_string(),
        description: String::new(),
        submit_label: String::new(),
        fields: vec![FormField {
            key: "name",
            label: "Name".to_string(),
            value: "a".to_string(),
            kind: FieldKind::Text,
            help: String::new(),
            cursor: 1,
        }],
        selected_field: 0,
        pending: PendingAction::SaveConfig,
    };
    app.set_canvas_mode(CanvasMode::EditForm(form));
    app.form_cursor_left();
    app.form_cursor_left();
    let CanvasMode::EditForm(form) = &app.canvas_mode else {
        panic!("expected EditForm");
    };
    assert_eq!(form.fields[0].cursor, 0);

    app.form_cursor_right();
    app.form_cursor_right();
    app.form_cursor_right();
    let CanvasMode::EditForm(form) = &app.canvas_mode else {
        panic!("expected EditForm");
    };
    assert_eq!(form.fields[0].cursor, 1);
}

#[test]
fn form_cursor_initialized_to_end_of_value() {
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
            cursor: 5,
        }],
        selected_field: 0,
        pending: PendingAction::SaveConfig,
    };
    app.set_canvas_mode(CanvasMode::EditForm(form));
    let CanvasMode::EditForm(form) = &app.canvas_mode else {
        panic!("expected EditForm");
    };
    assert_eq!(form.fields[0].cursor, 5);
}

#[test]
fn render_form_line_shows_cursor_in_text_field() {
    let field = FormField {
        key: "name",
        label: "Name".to_string(),
        value: "penpot".to_string(),
        kind: FieldKind::Text,
        help: String::new(),
        cursor: 6,
    };
    let line = render_form_line(&field, true);
    assert!(line.contains("penpot"));
    assert!(line.contains("▏"));
}

#[test]
fn render_form_line_shows_cursor_in_secret_field() {
    let field = FormField {
        key: "token",
        label: "PAT".to_string(),
        value: "secret".to_string(),
        kind: FieldKind::Secret,
        help: String::new(),
        cursor: 6,
    };
    let line = render_form_line(&field, true);
    assert!(line.contains("••••••"));
    assert!(line.contains("▏"));
}

#[test]
fn escape_in_form_mode_returns_to_browse() {
    let mut app = test_app();
    let form = FormState {
        title: "Test".to_string(),
        description: String::new(),
        submit_label: String::new(),
        fields: vec![FormField {
            key: "name",
            label: "Name".to_string(),
            value: String::new(),
            kind: FieldKind::Text,
            help: String::new(),
            cursor: 0,
        }],
        selected_field: 0,
        pending: PendingAction::SaveConfig,
    };
    app.set_canvas_mode(CanvasMode::EditForm(form));
    app.handle_key(crossterm::event::KeyCode::Esc);
    assert!(matches!(app.canvas_mode, CanvasMode::Browse));
}

#[test]
fn tab_in_form_mode_cycles_fields() {
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
                cursor: 0,
            },
            FormField {
                key: "b",
                label: "B".to_string(),
                value: String::new(),
                kind: FieldKind::Text,
                help: String::new(),
                cursor: 0,
            },
        ],
        selected_field: 0,
        pending: PendingAction::SaveConfig,
    };
    app.set_canvas_mode(CanvasMode::EditForm(form));
    app.handle_key(crossterm::event::KeyCode::Tab);
    let CanvasMode::EditForm(form) = &app.canvas_mode else {
        panic!("expected EditForm");
    };
    assert_eq!(form.selected_field, 1);
}

#[test]
fn filtered_directory_renders_callback_and_opens_its_owner_at_both_sizes() {
    use crossterm::event::KeyCode::*;
    use ratatui::{backend::TestBackend, Terminal};
    let mut app = test_app();
    app.app_records = vec![
        Record {
            name: "Other".into(),
            ..Default::default()
        },
        Record {
            id: "g".into(),
            name: "Grafana".into(),
            kind: "confidential".into(),
            redirect_uris: vec!["https://grafana.example/cb".into()],
            ..Default::default()
        },
    ];
    for key in [Char('/'), Char('G'), Char('r'), Enter, Enter, Down] {
        app.handle_key(key);
    }
    for (width, height) in [(100, 28), (40, 18)] {
        let mut term = Terminal::new(TestBackend::new(width, height)).unwrap();
        term.draw(|f| super::draw(f, &app, &mut vec![])).unwrap();
        let screen = term
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(screen.contains("https://grafana.example/cb"));
        assert!(screen.contains("Add redirect URIs"));
        assert!(!screen.contains("Other"));
    }
    assert!(
        matches!(app.handle_key(Enter),AppCommand::BeginAction{action_index:4,selected_record:Some(r),..} if r.id=="g")
    );
    app.handle_key(Esc);
    assert_eq!(app.expanded, None);
}

#[test]
fn settings_opens_connection_and_configuration() {
    use crossterm::event::KeyCode::*;
    let mut app = test_app();
    for key in [Up, Enter] {
        app.handle_key(key);
    }
    assert!(matches!(
        app.handle_key(Enter),
        AppCommand::BeginAction {
            resource: ResourceKind::Auth,
            ..
        }
    ));
    app.handle_key(Down);
    assert!(matches!(
        app.handle_key(Enter),
        AppCommand::BeginAction {
            resource: ResourceKind::Config,
            ..
        }
    ));
}

#[test]
fn wide_directory_joins_borders_and_exposes_provider_click_targets() {
    use ratatui::{backend::TestBackend, Terminal};
    let mut app = test_app();
    app.selected_resource = 2;
    app.idp_error = Some("Could not load identity providers: forbidden".into());
    let mut hits = vec![];
    let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
    terminal.draw(|f| super::draw(f, &app, &mut hits)).unwrap();
    let buf = terminal.backend().buffer();
    let border = buf[(20, 2)].fg;
    for (x, rows) in [(0, 0..24), (99, 0..24), (21, 2..22)] {
        for y in rows {
            assert_eq!(buf[(x, y)].bg, border);
            assert_eq!(buf[(x, y)].symbol(), " ");
        }
    }
    assert!(hits
        .iter()
        .any(|(r, t)| matches!(t, super::Target::Section(2)) && r.contains((3, 5).into())));
    let screen = buf.content.iter().map(|c| c.symbol()).collect::<String>();
    assert!(screen.contains("Could not load identity providers: forbidden"));
    assert!(!screen.contains("No records"));
}
