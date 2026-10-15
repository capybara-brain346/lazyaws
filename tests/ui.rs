#[allow(dead_code)]
mod common;

use std::time::Instant;

use common::{app_with_service, render, render_with_map, sample_operation, sample_resource};
use lazyaws::{
    app::{ActionDialog, Focus},
    config::Config,
    model::{Resource, Service},
    operations::{AccountContext, Parameter},
    ui::MouseTarget,
};

#[test]
fn renders_service_tabs_and_five_rounded_side_panels_for_each_service() {
    for service in Service::ALL {
        let app = app_with_service(service);
        let (text, _) = render(&app, &Config::default(), 120, 36);

        assert!(text.contains(service.name()));
        assert!(text.contains("EC2"));
        assert!(text.contains("ECS"));
        assert!(text.contains("ECR"));
        assert!(text.contains("S3"));
        assert!(text.contains("Status"));
        assert!(text.contains("[1] Status"));
        assert!(text.contains("[2] "));
        assert!(text.contains("[0] Details"));
        assert!(text.contains("[6] Command log"));
        for title in service.panel_titles() {
            assert!(
                text.contains(title),
                "missing {title} for {}",
                service.name()
            );
        }
        assert!(text.contains("Details"));
        assert!(text.contains("Command log"));
        assert!(text.contains('╭'));
        let compact_pane_row = text.lines().position(|line| line.contains("[5]")).unwrap();
        let log_row = text
            .lines()
            .position(|line| line.contains("[6] Command log"))
            .unwrap();
        assert_eq!(compact_pane_row, 32);
        assert_eq!(log_row, 32);

        let mut focused = app;
        focused.focus = Focus::List(3);
        let (expanded, _) = render(&focused, &Config::default(), 120, 36);
        let expanded_pane_row = expanded
            .lines()
            .position(|line| line.contains("[5]"))
            .unwrap();
        assert!(expanded_pane_row < compact_pane_row);
    }
}

#[test]
fn renders_selected_resource_json_status_and_sanitized_stale_errors() {
    let mut app = app_with_service(Service::Ec2);
    app.snapshot.panels[0].items.push(sample_resource());
    app.focus = Focus::List(0);
    app.detail_scroll = u16::MAX;
    app.error = Some("refresh failed\u{1b}[31m".into());
    app.logs.push_back("aws output\u{1b}[2J".into());
    let config = Config {
        profile: Some("dev".into()),
        region: Some("us-east-1".into()),
        demo: true,
        ..Config::default()
    };

    let (text, has_blue_list_row) = render(&app, &config, 120, 36);
    assert!(text.contains("Web server"));
    assert!(text.contains("instanceId"));
    assert!(text.contains("\"instanceId\": \"i-123\""));
    assert!(text.contains("Refresh failed; showing last data: refresh failed [31m"));
    assert!(text.contains("pf:dev"));
    assert!(text.contains("rg:us-east-1"));
    assert!(text.contains("DEMO"));
    assert!(!text.contains('\u{1b}'));
    assert!(has_blue_list_row);
}

#[test]
fn status_keeps_only_context_during_and_after_refresh() {
    let mut app = app_with_service(Service::Ec2);
    app.last_updated = Some(Instant::now() - std::time::Duration::from_secs(65));

    for loading in [false, true] {
        app.loading = loading;
        let (text, _) = render(&app, &Config::default(), 180, 36);
        let status = text
            .lines()
            .skip_while(|line| !line.contains("[1] Status"))
            .nth(1)
            .unwrap()
            .split('│')
            .nth(1)
            .unwrap()
            .trim();

        assert_eq!(status, "pf:AWS chain rg:AWS config");
    }
}

#[test]
fn scrolls_the_selected_resource_into_the_list_view() {
    let mut app = app_with_service(Service::Ec2);
    app.snapshot.panels[0].items = (0..40)
        .map(|index| Resource {
            id: format!("i-{index}"),
            label: format!("resource-{index}"),
            summary: "running".into(),
            detail: serde_json::json!({"name": format!("resource-{index}")}),
        })
        .collect();
    app.selected[0] = 39;
    app.focus = Focus::List(0);

    let (text, has_blue_list_row) = render(&app, &Config::default(), 120, 36);
    assert!(text.contains("resource-39"));
    assert!(!text.contains("resource-0"));
    assert!(has_blue_list_row);
}

#[test]
fn help_overlay_and_small_terminal_show_useful_text() {
    let mut app = app_with_service(Service::S3);
    app.help = true;
    let (help, _) = render(&app, &Config::default(), 110, 36);
    assert!(help.contains("Keybindings"));
    assert!(help.contains("2–5 lists"));
    assert!(help.contains("PgUp/PgDn"));

    app.help = false;
    app.filter = "prod".into();
    let (filtered, _) = render(&app, &Config::default(), 110, 36);
    assert!(filtered.contains("Filtered: prod"));

    app.filter.clear();
    let (warning, _) = render(&app, &Config::default(), 30, 8);
    assert!(warning.contains("Terminal"));
    assert!(warning.contains("too small"));
}

#[test]
fn hit_map_uses_tab_footer_and_visible_row_coordinates() {
    let mut app = app_with_service(Service::Ec2);
    app.snapshot.panels[0].items = (0..40)
        .map(|index| Resource {
            id: format!("i-{index}"),
            label: format!("needle-{index}"),
            summary: "running".into(),
            detail: serde_json::json!({"name": format!("needle-{index}")}),
        })
        .collect();
    app.snapshot.panels[0].error = Some("refresh failed".into());
    app.filter = "needle-1".into();
    app.selected[0] = 19;
    app.focus = Focus::List(0);

    let (text, hit_map) = render_with_map(&app, &Config::default(), 120, 36);
    assert!(text.contains("needle-19"));
    assert!(!text.contains("needle-0"));
    assert_eq!(
        hit_map.target_at(10, 0),
        Some(MouseTarget::Service(Service::Ec2))
    );
    assert_eq!(
        hit_map.target_at(15, 0),
        Some(MouseTarget::Service(Service::Ecs))
    );
    assert_eq!(hit_map.target_at(3, 35), Some(MouseTarget::OpenActions));

    let (row, target) = hit_map
        .regions
        .iter()
        .find(|(_, target)| {
            *target
                == MouseTarget::Row {
                    panel: 0,
                    index: 19,
                }
        })
        .copied()
        .expect("selected filtered row is clickable");
    assert_eq!(hit_map.target_at(row.x, row.y), Some(target));
}

#[test]
fn compact_layout_keeps_resource_rows_clickable() {
    let mut app = app_with_service(Service::S3);
    app.focus = Focus::List(0);
    app.snapshot.panels[0].items.push(sample_resource());

    let (_, hit_map) = render_with_map(&app, &Config::default(), 60, 20);
    let (row, target) = hit_map
        .regions
        .iter()
        .find(|(_, target)| {
            matches!(
                target,
                MouseTarget::Row {
                    panel: 0,
                    index: 0,
                    ..
                }
            )
        })
        .copied()
        .expect("compact layout row is clickable");
    assert_eq!(target, MouseTarget::Row { panel: 0, index: 0 });
    assert_eq!(hit_map.target_at(row.x, row.y), Some(target));
}

#[test]
fn account_header_shows_identity_or_failure_without_stale_identity() {
    let mut app = app_with_service(Service::Ec2);
    app.account = Some(Ok(AccountContext {
        account: "123456789012".into(),
        arn: "arn:aws:sts::123456789012:assumed-role/Team/read-only".into(),
        user_id: "AROATEST:read-only".into(),
    }));
    let (text, _) = render_with_map(&app, &Config::default(), 60, 20);
    assert!(text.contains("123456789012"));
    assert!(text.contains("assumed-role/Team/read-only"));

    app.focus = Focus::Status;
    let (details, _) = render_with_map(&app, &Config::default(), 100, 30);
    assert!(details.contains("arn:aws:sts::123456789012:assumed-role/Team/read-only"));
    assert!(details.contains("AROATEST:read-only"));
    assert!(!details.contains("AKIA"));

    app.account = Some(Err("credentials expired".into()));
    let (failed, _) = render_with_map(&app, &Config::default(), 100, 30);
    assert!(failed.contains("Account: unavailable"));
    assert!(failed.contains("credentials expired"));
    assert!(!failed.contains("123456789012"));
}

#[test]
fn action_dialog_shows_target_warning_command_and_click_targets_at_minimum_size() {
    let mut app = app_with_service(Service::Ec2);
    app.actions = Some(ActionDialog {
        operations: vec![sample_operation(None)],
        selected: 0,
        confirming: true,
        editing: false,
        input: String::new(),
        error: None,
        preview: sample_operation(None).args(""),
    });
    app.account = Some(Ok(AccountContext {
        account: "123456789012".into(),
        arn: "arn:aws:iam::123456789012:user/dev".into(),
        user_id: "AIDAEXAMPLE".into(),
    }));
    let config = Config {
        profile: Some("prod".into()),
        region: Some("eu-west-1".into()),
        ..Config::default()
    };

    let (text, hit_map) = render_with_map(&app, &config, 60, 20);
    assert!(text.contains("Stop instance"));
    assert!(text.contains("i-123"));
    assert!(text.contains("interrupt work"));
    assert!(text.contains("123456789012"));
    assert!(text.contains("prod"));
    assert!(text.contains("eu-west-1"));
    assert!(text.contains("stop-instances"));
    assert!(text.contains("Click Y for yes, N for cancel, or Escape to cancel."));
    assert!(text.contains("[ Y ]"));
    assert!(text.contains("[ N ]"));
    assert!(!text.contains("[ Run ]"));
    assert!(!text.contains("[ Cancel ]"));

    for target in [MouseTarget::ConfirmAction, MouseTarget::CancelAction] {
        let (area, _) = hit_map
            .regions
            .iter()
            .find(|(_, region_target)| *region_target == target)
            .copied()
            .expect("dialog control is clickable");
        assert_eq!(hit_map.target_at(area.x, area.y), Some(target));
    }
}

#[test]
fn executing_action_shows_wait_state_without_confirmation_controls() {
    let mut app = app_with_service(Service::Ec2);
    app.actions = Some(ActionDialog {
        operations: vec![sample_operation(None)],
        selected: 0,
        confirming: true,
        editing: false,
        input: String::new(),
        error: None,
        preview: Ok(Vec::new()),
    });
    app.executing = true;

    let (text, hit_map) = render_with_map(&app, &Config::default(), 60, 20);
    assert!(text.contains("Running action"));
    assert!(text.contains("wait for the result"));
    assert!(!text.contains("[ Y ]"));
    assert!(!text.contains("[ N ]"));
    assert!(!text.contains("[ Cancel ]"));
    assert!(!hit_map.regions.iter().any(|(_, target)| {
        matches!(
            target,
            MouseTarget::ConfirmAction | MouseTarget::CancelAction
        )
    }));
}

#[test]
fn action_menu_rows_are_clickable_at_minimum_size() {
    let mut app = app_with_service(Service::Ec2);
    let mut second = sample_operation(None);
    second.label = "Reboot instance";
    second.name = "reboot-instances";
    app.actions = Some(ActionDialog {
        operations: vec![sample_operation(None), second],
        selected: 0,
        confirming: false,
        editing: false,
        input: String::new(),
        error: None,
        preview: Ok(Vec::new()),
    });

    let (text, hit_map) = render_with_map(&app, &Config::default(), 60, 20);
    assert!(text.contains("Choose action"));
    assert!(text.contains("Stop instance"));
    assert!(text.contains("Reboot instance"));
    let (area, target) = hit_map
        .regions
        .iter()
        .find(|(_, target)| *target == MouseTarget::ChooseAction(1))
        .copied()
        .expect("second action is clickable");
    assert_eq!(hit_map.target_at(area.x, area.y), Some(target));
}

#[test]
fn parameter_dialog_shows_input_error_and_accept_button() {
    let mut app = app_with_service(Service::S3);
    app.actions = Some(ActionDialog {
        operations: vec![sample_operation(Some(Parameter::UploadFilePath))],
        selected: 0,
        confirming: false,
        editing: true,
        input: String::new(),
        error: Some("Upload file path is required".into()),
        preview: Ok(Vec::new()),
    });

    let (text, hit_map) = render_with_map(&app, &Config::default(), 60, 20);
    assert!(text.contains("Upload file path"));
    assert!(text.contains("Upload file path is required"));
    assert!(text.contains("Accept"));
    assert!(!text.contains("[ Run ]"));
    let (area, _) = hit_map
        .regions
        .iter()
        .find(|(_, target)| *target == MouseTarget::ConfirmAction)
        .copied()
        .expect("accept control is clickable");
    assert_eq!(
        hit_map.target_at(area.x, area.y),
        Some(MouseTarget::ConfirmAction)
    );
}

#[test]
fn long_action_menu_keeps_the_selected_row_visible_and_clickable() {
    let mut app = app_with_service(Service::S3);
    let mut last = sample_operation(None);
    last.label = "Last bucket action";
    let mut operations = vec![sample_operation(None); 20];
    operations.push(last);
    app.actions = Some(ActionDialog {
        operations,
        selected: 20,
        confirming: false,
        editing: false,
        input: String::new(),
        error: None,
        preview: Ok(Vec::new()),
    });

    let (text, hit_map) = render_with_map(&app, &Config::default(), 60, 20);
    assert!(text.contains("Last bucket action"));
    let (area, _) = hit_map
        .regions
        .iter()
        .find(|(_, target)| *target == MouseTarget::ChooseAction(20))
        .expect("selected action is clickable");
    assert_eq!(
        hit_map.target_at(area.x, area.y),
        Some(MouseTarget::ChooseAction(20))
    );
}
