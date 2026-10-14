use std::time::Instant;

use lazyaws::{
    app::App,
    config::{Config, SIDE_PANEL_PERCENT},
    model::{Resource, Service, Snapshot},
    ui,
};
use ratatui::{Terminal, backend::TestBackend, style::Color};

fn app_with_service(service: Service) -> App {
    let mut app = App::new();
    app.service = service;
    app.snapshot = Snapshot::empty(service);
    app.loading = false;
    app.last_updated = Some(Instant::now());
    app
}

fn sample_resource() -> Resource {
    Resource {
        id: "i-123".into(),
        label: "Web server".into(),
        summary: "running".into(),
        detail: serde_json::json!({
            "instanceId": "i-123",
            "state": "running",
            "size": 2,
        }),
    }
}

fn render(app: &App, config: &Config, width: u16, height: u16) -> (String, bool) {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| ui::draw(frame, app, config)).unwrap();

    let buffer = terminal.backend().buffer();
    let text = (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    let side_width = width * SIDE_PANEL_PERCENT / 100;
    let has_blue_list_row =
        (5..height).any(|y| (0..side_width).any(|x| buffer[(x, y)].bg == Color::Blue));
    (text, has_blue_list_row)
}

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
    }
}

#[test]
fn renders_selected_resource_json_status_and_sanitized_stale_errors() {
    let mut app = app_with_service(Service::Ec2);
    app.snapshot.panels[0].items.push(sample_resource());
    app.focus = 1;
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
    assert!(text.contains("5s"));
    assert!(!text.contains('\u{1b}'));
    assert!(has_blue_list_row);
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
    app.focus = 1;

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
