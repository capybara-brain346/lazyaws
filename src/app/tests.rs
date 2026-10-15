use serde_json::json;

use super::*;
use crate::{
    model::{Resource, Service, Snapshot},
    operations::{AccountContext, Parameter},
    ui::{HitMap, MouseTarget},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn resource(id: &str, label: &str) -> Resource {
    Resource {
        id: id.into(),
        label: label.into(),
        summary: label.into(),
        detail: json!({"name": label}),
    }
}

fn snapshot(primary: &[(&str, &str)], related: &[(&str, &str)]) -> Snapshot {
    let mut snapshot = Snapshot::empty(Service::Ec2);
    snapshot.panels[0].items = primary
        .iter()
        .map(|(id, label)| resource(id, label))
        .collect();
    snapshot.panels[1].items = related
        .iter()
        .map(|(id, label)| resource(id, label))
        .collect();
    snapshot
}

#[test]
fn primary_navigation_refreshes_and_clears_related_data() {
    let mut app = App::new();
    app.apply(snapshot(
        &[("a", "alpha"), ("b", "beta")],
        &[("v1", "volume")],
    ));
    app.handle_key(key(KeyCode::Char('2')));

    assert_eq!(app.handle_key(key(KeyCode::Down)), Action::Refresh);
    assert_eq!(app.selected_primary(), Some("b"));
    assert!(app.snapshot.panels[1].items.is_empty());
}

#[test]
fn filter_uses_raw_indexes_and_treats_q_and_r_as_text() {
    let mut app = App::new();
    app.apply(snapshot(
        &[("0", "quiet"), ("1", "other"), ("2", "query result")],
        &[("volume", "attached volume")],
    ));
    app.handle_key(key(KeyCode::Char('2')));
    app.handle_key(key(KeyCode::Char('/')));
    assert_eq!(app.handle_key(key(KeyCode::Char('q'))), Action::None);
    assert_eq!(app.handle_key(key(KeyCode::Char('r'))), Action::None);
    assert_eq!(app.filter, "qr");
    app.handle_key(key(KeyCode::Esc));
    assert!(app.filter.is_empty());

    app.handle_key(key(KeyCode::Char('/')));
    app.handle_key(key(KeyCode::Char('q')));
    assert_eq!(app.visible_indices(0), vec![0, 2]);
    assert_eq!(app.visible_indices(1), vec![0]);
    assert_eq!(app.selected[0], 0);
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.handle_key(key(KeyCode::Down)), Action::Refresh);
    assert_eq!(app.selected[0], 2);
}

#[test]
fn tabs_preserve_each_service_filter_and_selection() {
    let mut app = App::new();
    app.apply(snapshot(&[("a", "alpha"), ("b", "beta")], &[]));
    app.handle_key(key(KeyCode::Char('2')));
    assert_eq!(app.handle_key(key(KeyCode::Down)), Action::Refresh);
    app.handle_key(key(KeyCode::Char('/')));
    app.handle_key(key(KeyCode::Char('b')));
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.handle_key(key(KeyCode::Char(']'))), Action::Refresh);
    assert_eq!(app.service, Service::Ecs);
    assert_eq!(app.handle_key(key(KeyCode::Char('['))), Action::Refresh);

    assert_eq!(app.service, Service::Ec2);
    assert_eq!(app.selected[0], 1);
    assert_eq!(app.filter, "b");
    assert_eq!(app.selected_primary(), Some("b"));
    app.apply(snapshot(
        &[("a", "alpha"), ("b", "beta")],
        &[("v2", "volume")],
    ));
    assert_eq!(app.selected[0], 1);
    assert_eq!(app.selected_primary(), Some("b"));
}

#[test]
fn apply_preserves_resource_identity_and_retains_empty_error_panels() {
    let mut app = App::new();
    app.apply(snapshot(
        &[("a", "alpha"), ("b", "beta")],
        &[("v1", "volume")],
    ));
    app.handle_key(key(KeyCode::Char('2')));
    app.handle_key(key(KeyCode::Down));
    app.apply(snapshot(
        &[("b", "beta"), ("a", "alpha")],
        &[("v1", "volume")],
    ));
    assert_eq!(app.selected[0], 0);
    assert_eq!(app.selected_primary(), Some("b"));

    app.apply(snapshot(&[("a", "alpha")], &[("v2", "new volume")]));
    assert_eq!(app.selected_primary(), Some("a"));
    assert_eq!(app.snapshot.panels[1].items[0].id, "v2");

    let mut failed = snapshot(&[("a", "alpha")], &[]);
    failed.panels[1].title = "Volumes".into();
    failed.panels[1].error = Some("request failed".into());
    app.apply(failed);
    assert_eq!(app.snapshot.panels[1].items[0].id, "v2");
    assert_eq!(
        app.snapshot.panels[1].error.as_deref(),
        Some("request failed")
    );
    assert!(app.error.as_deref().unwrap().contains("request failed"));
}

#[test]
fn changed_parent_uses_partial_results_and_drops_old_context_on_error() {
    let mut app = App::new();
    app.apply(snapshot(
        &[("a", "alpha"), ("b", "beta")],
        &[("v1", "volume")],
    ));
    let mut partial = snapshot(&[("a", "alpha"), ("b", "beta")], &[("v2", "new volume")]);
    partial.panels[1].error = Some("volume query partly failed".into());
    app.apply(partial);
    assert_eq!(app.snapshot.panels[1].items[0].id, "v2");

    app.handle_key(key(KeyCode::Char('2')));
    assert_eq!(app.handle_key(key(KeyCode::Down)), Action::Refresh);
    assert!(app.snapshot.panels[1].items.is_empty());

    let mut partial = snapshot(&[("b", "beta")], &[]);
    partial.panels[1].error = Some("volume query failed".into());
    app.apply(partial);
    assert!(app.snapshot.panels[1].items.is_empty());
    assert!(app.snapshot.panels[1].error.is_some());
}

#[test]
fn primary_list_error_keeps_and_marks_related_data_stale() {
    let mut app = App::new();
    app.apply(snapshot(&[("a", "alpha")], &[("v1", "volume")]));
    let mut failed = snapshot(&[], &[]);
    failed.panels[0].error = Some("instance list failed".into());
    app.apply(failed);

    assert_eq!(app.selected_primary(), Some("a"));
    assert_eq!(app.snapshot.panels[1].items[0].id, "v1");
    assert_eq!(
        app.snapshot.panels[1].error.as_deref(),
        Some("instance list failed")
    );
}

#[test]
fn detail_focus_returns_to_its_list_and_scrolls() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('1')));
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.focus, Focus::List(0));
    app.handle_key(key(KeyCode::BackTab));
    assert_eq!(app.focus, Focus::Status);
    app.apply(snapshot(&[("a", "alpha")], &[]));
    app.handle_key(key(KeyCode::Char('3')));
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.focus, Focus::Detail);
    app.handle_key(key(KeyCode::PageDown));
    assert_eq!(app.detail_scroll, 10);
    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.focus, Focus::List(1));
}

#[test]
fn mouse_selects_raw_filtered_rows_and_scrolls_the_hovered_pane() {
    let mut app = App::new();
    app.apply(snapshot(
        &[("0", "quiet"), ("1", "other"), ("2", "query result")],
        &[("v1", "volume")],
    ));
    app.focus = Focus::List(1);
    let details = HitMap {
        regions: vec![(Rect::new(0, 0, 10, 5), MouseTarget::Pane(Focus::Detail))],
    };
    let click_details = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 1,
        row: 1,
        modifiers: KeyModifiers::NONE,
    };
    assert_eq!(app.handle_mouse(click_details, &details), Action::None);
    assert_eq!(
        app.selected_resource().map(|resource| resource.id.as_str()),
        Some("v1")
    );
    app.focus = Focus::List(1);
    let scroll_details = MouseEvent {
        kind: MouseEventKind::ScrollDown,
        ..click_details
    };
    assert_eq!(app.handle_mouse(scroll_details, &details), Action::None);
    assert_eq!(
        app.selected_resource().map(|resource| resource.id.as_str()),
        Some("v1")
    );
    assert_eq!(app.detail_scroll, 10);

    app.focus = Focus::List(0);
    app.filter = "q".into();
    app.filter_panel = 0;
    assert_eq!(app.visible_indices(0), vec![0, 2]);

    let rect = Rect::new(0, 0, 10, 5);
    let targets = HitMap {
        regions: vec![
            (rect, MouseTarget::Pane(Focus::List(0))),
            (rect, MouseTarget::Row { panel: 0, index: 2 }),
        ],
    };
    assert_eq!(
        targets.target_at(1, 1),
        Some(MouseTarget::Row { panel: 0, index: 2 })
    );
    let click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 1,
        row: 1,
        modifiers: KeyModifiers::NONE,
    };
    assert_eq!(app.handle_mouse(click, &targets), Action::Refresh);
    assert_eq!(app.selected[0], 2);
    assert!(app.snapshot.panels[1].items.is_empty());

    let pane = HitMap {
        regions: vec![(rect, MouseTarget::Pane(Focus::List(0)))],
    };
    let scroll = MouseEvent {
        kind: MouseEventKind::ScrollUp,
        ..click
    };
    assert_eq!(app.handle_mouse(scroll, &pane), Action::Refresh);
    assert_eq!(app.selected[0], 0);
}

fn ecs_app() -> App {
    let mut app = App::new();
    app.service = Service::Ecs;
    app.apply(ecs_snapshot());
    app.account = Some(Ok(AccountContext {
        account: "123456789012".into(),
        arn: "arn:aws:iam::123456789012:user/demo".into(),
        user_id: "AIDADEMO".into(),
    }));
    app.focus = Focus::List(1);
    app
}

fn ecs_snapshot() -> Snapshot {
    let mut data = Snapshot::empty(Service::Ecs);
    data.panels[0].items.push(Resource {
        id: "cluster-a".into(),
        label: "cluster-a".into(),
        summary: "ACTIVE".into(),
        detail: json!({"clusterArn":"cluster-a"}),
    });
    data.panels[1].items.push(Resource {
        id: "api".into(),
        label: "api".into(),
        summary: "2/2 running".into(),
        detail: json!({"serviceName":"api","desiredCount":2}),
    });
    data
}

#[test]
fn action_parameter_requires_lock_then_y_and_keeps_the_captured_target() {
    let mut app = ecs_app();
    app.open_actions();
    assert_eq!(
        app.actions.as_ref().unwrap().operations[1].parameter,
        Some(Parameter::DesiredTaskCount)
    );
    assert_eq!(app.handle_key(key(KeyCode::Char('j'))), Action::None);
    assert_eq!(app.handle_key(key(KeyCode::Enter)), Action::None);
    assert!(app.actions.as_ref().unwrap().editing);
    assert_eq!(app.actions.as_ref().unwrap().input, "2");
    app.handle_key(key(KeyCode::Backspace));

    for character in ['q', 'r', 'y'] {
        assert_eq!(app.handle_key(key(KeyCode::Char(character))), Action::None);
    }
    assert_eq!(app.actions.as_ref().unwrap().input, "qry");
    for _ in 0..3 {
        app.handle_key(key(KeyCode::Backspace));
    }

    app.handle_key(key(KeyCode::Char('-')));
    app.handle_key(key(KeyCode::Char('1')));
    assert!(app.actions.as_ref().unwrap().preview.is_err());
    app.handle_key(key(KeyCode::Enter));
    let dialog = app.actions.as_ref().unwrap();
    assert!(dialog.editing);
    assert!(dialog.error.is_some());

    app.handle_key(key(KeyCode::Backspace));
    app.handle_key(key(KeyCode::Backspace));
    app.handle_key(key(KeyCode::Char('3')));
    app.handle_key(key(KeyCode::Enter));
    assert!(app.actions.as_ref().unwrap().confirming);
    assert!(!app.actions.as_ref().unwrap().editing);

    app.snapshot.panels[0].items[0].id = "cluster-changed".into();
    assert_eq!(app.handle_key(key(KeyCode::Enter)), Action::None);
    let Action::Execute(operation, value) = app.handle_key(key(KeyCode::Char('y'))) else {
        panic!("expected confirmed action");
    };
    assert_eq!(value, "3");
    assert_eq!(operation.target, "api");
    assert_eq!(operation.input["cluster"], "cluster-a");
    assert!(app.executing);
    assert_eq!(app.handle_key(key(KeyCode::Char(']'))), Action::None);
    assert_eq!(app.service, Service::Ecs);
}

#[test]
fn action_dialog_cancels_and_does_not_run_on_a_second_enter() {
    let mut app = ecs_app();
    app.open_actions();
    assert_eq!(app.handle_key(key(KeyCode::Esc)), Action::None);
    assert!(app.actions.is_none());

    app.open_actions();
    assert_eq!(app.handle_key(key(KeyCode::Enter)), Action::None);
    assert!(app.actions.as_ref().unwrap().confirming);
    assert_eq!(app.handle_key(key(KeyCode::Enter)), Action::None);
    assert!(app.actions.is_some());
    assert_eq!(app.handle_key(key(KeyCode::Esc)), Action::None);
    assert!(app.actions.is_none());

    for character in ['n', 'N'] {
        app.open_actions();
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.handle_key(key(KeyCode::Char(character))), Action::None);
        assert!(app.actions.is_none());
        assert!(!app.executing);
    }

    for character in ['y', 'Y'] {
        app.open_actions();
        app.handle_key(key(KeyCode::Enter));
        assert!(matches!(
            app.handle_key(key(KeyCode::Char(character))),
            Action::Execute(_, _)
        ));
        assert!(app.executing);
        app.executing = false;
        app.actions = None;
    }

    app.operation_output = Some("completed".into());
    let mut refreshed = ecs_snapshot();
    refreshed.panels[1].items.clear();
    app.apply(refreshed);
    assert_eq!(app.operation_output.as_deref(), Some("completed"));
}

#[test]
fn action_and_help_overlays_block_mouse_click_through() {
    let mut app = ecs_app();
    app.open_actions();
    let targets = HitMap {
        regions: vec![(Rect::new(0, 0, 5, 5), MouseTarget::Service(Service::Ec2))],
    };
    let click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 1,
        row: 1,
        modifiers: KeyModifiers::NONE,
    };
    assert_eq!(app.handle_mouse(click, &targets), Action::None);
    assert_eq!(app.service, Service::Ecs);

    app.actions = None;
    app.help = true;
    assert_eq!(app.handle_mouse(click, &targets), Action::None);
    assert_eq!(app.service, Service::Ecs);
}

#[test]
fn theme_picker_previews_then_applies_or_reverts() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('t')));
    app.handle_key(key(KeyCode::Down));

    assert_eq!(app.theme().name, "Catppuccin Mocha");
    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.theme().name, "Terminal");

    app.handle_key(key(KeyCode::Char('t')));
    app.handle_key(key(KeyCode::Up));
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.theme().name, "Cobalt2");
    assert_eq!(app.theme_picker, None);
}
