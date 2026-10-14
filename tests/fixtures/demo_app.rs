use serde_json::json;

use super::{Action, App};
use crate::model::{Resource, Service, Snapshot};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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
    assert_eq!(app.visible_indices(1), vec![0, 2]);
    assert_eq!(app.visible_indices(2), vec![0]);
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
    assert_eq!(app.focus, 1);
    app.handle_key(key(KeyCode::BackTab));
    assert_eq!(app.focus, 0);
    app.apply(snapshot(&[("a", "alpha")], &[]));
    app.handle_key(key(KeyCode::Char('3')));
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.focus, 5);
    app.handle_key(key(KeyCode::PageDown));
    assert_eq!(app.detail_scroll, 10);
    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.focus, 2);
}
