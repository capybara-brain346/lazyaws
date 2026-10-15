use crossterm::event::KeyModifiers;
use ratatui::{Terminal, backend::TestBackend};

use super::*;
use crate::{
    app::ActionDialog,
    config::{Config, DEFAULT_THEME},
    demo,
    model::Service,
    operations::Operation,
    ui::{self, MouseTarget},
};

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

#[test]
fn click_runs_on_release_but_drag_over_a_confirmation_button_only_copies() {
    let screen = Buffer::with_lines(["[ Y ] [ N ]"]);
    let targets = HitMap {
        regions: vec![(screen.area, MouseTarget::ConfirmAction)],
    };
    let mut app = App::new();
    app.actions = Some(ActionDialog {
        operations: vec![Operation {
            label: "Stop instance",
            service: "ec2",
            name: "stop-instances",
            input: serde_json::json!({"InstanceIds": ["i-test"]}),
            region: None,
            target: "i-test".into(),
            warning: "Stops the instance.",
            parameter: None,
        }],
        selected: 0,
        confirming: true,
        editing: false,
        input: String::new(),
        error: None,
    });
    let mut selection = TextSelection::default();
    let down = mouse(MouseEventKind::Down(MouseButton::Left), 0, 0);
    assert_eq!(
        selection.handle_mouse(down, &screen, &mut app, &targets),
        Action::None
    );
    assert!(!app.executing);
    selection.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), 4, 0),
        &screen,
        &mut app,
        &targets,
    );
    assert_eq!(
        selection.handle_mouse(
            mouse(MouseEventKind::Up(MouseButton::Left), 4, 0),
            &screen,
            &mut app,
            &targets,
        ),
        Action::Copy("[ Y ]".into())
    );
    assert!(!app.executing);

    selection.handle_mouse(down, &screen, &mut app, &targets);
    assert!(matches!(
        selection.handle_mouse(
            mouse(MouseEventKind::Up(MouseButton::Left), 0, 0),
            &screen,
            &mut app,
            &targets,
        ),
        Action::Execute(_, _)
    ));
}

#[test]
fn reverse_rectangle_keeps_unicode_and_copies_the_highlighted_snapshot() {
    let screen = Buffer::with_lines(["界e\u{301}  x", "abc  y"]);
    let mut selection = TextSelection::default();
    let mut app = App::new();
    app.executing = true;
    let targets = HitMap::default();
    selection.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), 4, 1),
        &screen,
        &mut app,
        &targets,
    );
    selection.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), 1, 0),
        &screen,
        &mut app,
        &targets,
    );

    let mut terminal = Terminal::new(TestBackend::new(6, 2)).unwrap();
    terminal
        .draw(|frame| assert!(selection.draw(frame, &DEFAULT_THEME)))
        .unwrap();
    let highlighted = terminal.backend().buffer();
    assert_eq!(highlighted[(2, 0)].symbol(), "e\u{301}");
    assert_eq!(
        highlighted[(2, 0)].bg,
        DEFAULT_THEME.text_selection_background
    );
    assert_ne!(
        highlighted[(5, 0)].bg,
        DEFAULT_THEME.text_selection_background
    );

    let changed = Buffer::with_lines(["xxxxxx", "yyyyyy"]);
    assert_eq!(
        selection.handle_mouse(
            mouse(MouseEventKind::Up(MouseButton::Left), 1, 0),
            &changed,
            &mut app,
            &targets,
        ),
        Action::Copy("界e\u{301}\nbc".into())
    );
}

#[test]
fn blank_selection_and_cancelled_or_resized_drag_do_not_copy_or_click() {
    let screen = Buffer::with_lines(["    ", "    "]);
    let mut app = App::new();
    let targets = HitMap {
        regions: vec![(screen.area, MouseTarget::Service(Service::Ecs))],
    };
    let mut selection = TextSelection::default();
    let down = mouse(MouseEventKind::Down(MouseButton::Left), 0, 0);
    let up = mouse(MouseEventKind::Up(MouseButton::Left), u16::MAX, u16::MAX);
    selection.handle_mouse(down, &screen, &mut app, &targets);
    assert_eq!(
        selection.handle_mouse(up, &screen, &mut app, &targets),
        Action::None
    );
    assert_eq!(app.service, Service::Ec2);

    selection.handle_mouse(down, &screen, &mut app, &targets);
    selection.clear();
    assert_eq!(
        selection.handle_mouse(up, &screen, &mut app, &targets),
        Action::None
    );

    selection.handle_mouse(down, &screen, &mut app, &targets);
    let mut terminal = Terminal::new(TestBackend::new(3, 1)).unwrap();
    terminal
        .draw(|frame| assert!(!selection.draw(frame, &DEFAULT_THEME)))
        .unwrap();
    assert_eq!(
        selection.handle_mouse(up, &screen, &mut app, &targets),
        Action::None
    );
    assert_eq!(app.service, Service::Ec2);
}

#[test]
fn data_in_every_service_can_be_copied_without_changing_the_selected_row() {
    for service in Service::ALL {
        let mut app = App::new();
        app.service = service;
        app.apply(demo::snapshot(service, None));
        let mut terminal = Terminal::new(TestBackend::new(120, 36)).unwrap();
        let mut targets = HitMap::default();
        let screen = terminal
            .draw(|frame| targets = ui::draw_interactive(frame, &app, &Config::default()))
            .unwrap();
        let (row, _) = targets
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
            .unwrap();
        let mut selection = TextSelection::default();
        selection.handle_mouse(
            mouse(MouseEventKind::Down(MouseButton::Left), row.x, row.y),
            screen.buffer,
            &mut app,
            &targets,
        );
        let action = selection.handle_mouse(
            mouse(
                MouseEventKind::Up(MouseButton::Left),
                row.right() - 1,
                row.y,
            ),
            screen.buffer,
            &mut app,
            &targets,
        );
        assert!(matches!(action, Action::Copy(text) if !text.is_empty()));
        assert_eq!(app.selected[0], 0);
    }
}
