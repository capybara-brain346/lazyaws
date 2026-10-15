use std::time::Instant;

use lazyaws::{
    app::App,
    config::{Config, SIDE_PANEL_WIDTH_DENOMINATOR, SIDE_PANEL_WIDTH_NUMERATOR},
    model::{Resource, Service, Snapshot},
    operations::Operation,
    ui::{self, HitMap},
};
use ratatui::{Terminal, backend::TestBackend, style::Color};

pub fn app_with_service(service: Service) -> App {
    let mut app = App::new();
    app.service = service;
    app.snapshot = Snapshot::empty(service);
    app.loading = false;
    app.last_updated = Some(Instant::now());
    app
}

pub fn sample_resource() -> Resource {
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

pub fn render(app: &App, config: &Config, width: u16, height: u16) -> (String, bool) {
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
    let side_width =
        (u32::from(width) * SIDE_PANEL_WIDTH_NUMERATOR / SIDE_PANEL_WIDTH_DENOMINATOR) as u16;
    let has_blue_list_row =
        (5..height).any(|y| (0..side_width).any(|x| buffer[(x, y)].bg == Color::Blue));
    (text, has_blue_list_row)
}

pub fn render_with_map(app: &App, config: &Config, width: u16, height: u16) -> (String, HitMap) {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut hit_map = HitMap::default();
    terminal
        .draw(|frame| {
            hit_map = ui::draw_interactive(frame, app, config);
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let text = (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    (text, hit_map)
}

pub fn sample_operation(parameter: Option<&'static str>) -> Operation {
    Operation {
        label: "Stop instance",
        service: "ec2",
        name: "stop-instances",
        input: serde_json::json!({"InstanceIds":["i-123"]}),
        region: None,
        target: "i-123".into(),
        warning: "Stops the instance and can interrupt work.",
        parameter,
    }
}
