#[allow(dead_code)]
mod common;

use common::{app_with_service, render_with_map};
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use lazyaws::{
    app::Action, config::Config, demo, model::Service, operations::AccountContext, ui::MouseTarget,
};

#[test]
fn actions_button_opens_for_a_selected_row_during_refresh() {
    let config = Config::default();
    let mut app = app_with_service(Service::Ec2);
    app.apply(demo::snapshot(Service::Ec2, None));
    app.account = Some(Ok(AccountContext {
        account: "123456789012".into(),
        arn: "arn:aws:iam::123456789012:user/demo".into(),
        user_id: "AIDADEMO".into(),
    }));
    let (_, hit_map) = render_with_map(&app, &config, 120, 36);
    let (row, _) = hit_map
        .regions
        .iter()
        .find(|(_, target)| {
            matches!(
                target,
                MouseTarget::Row {
                    panel: 0,
                    index: 1,
                    ..
                }
            )
        })
        .expect("second resource row is clickable");
    let mut click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: row.x,
        row: row.y,
        modifiers: KeyModifiers::NONE,
    };

    assert_eq!(app.handle_mouse(click, &hit_map), Action::Refresh);
    assert!(app.loading);
    let target = app.selected_primary().unwrap().to_owned();
    let (_, hit_map) = render_with_map(&app, &config, 120, 36);
    let (button, _) = hit_map
        .regions
        .iter()
        .find(|(_, target)| *target == MouseTarget::OpenActions)
        .expect("actions button is clickable");
    click.column = button.x;
    click.row = button.y;

    assert_eq!(app.handle_mouse(click, &hit_map), Action::None);
    let dialog = app
        .actions
        .as_ref()
        .expect("actions dialog opens during refresh");
    assert!(
        dialog
            .operations
            .iter()
            .all(|operation| operation.target == target)
    );
    assert!(!app.executing);
    let (text, _) = render_with_map(&app, &config, 120, 36);
    assert!(text.contains("Choose action"));
}
