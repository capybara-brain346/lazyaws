use std::{borrow::Cow, str::FromStr};

use ratatui::style::{Color, Style};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    pub name: Cow<'static, str>,
    pub appearance: Cow<'static, str>,
    pub background: Color,
    pub text: Color,
    pub border: Color,
    pub focus: Color,
    pub title: Color,
    pub muted: Color,
    pub key: Color,
    pub warning: Color,
    pub error: Color,
    pub selection_text: Color,
    pub selection_background: Color,
    pub inactive_selection_background: Color,
    pub text_selection_text: Color,
    pub text_selection_background: Color,
    pub json_key: Color,
    pub json_string: Color,
    pub json_literal: Color,
    pub json_number: Color,
}

impl Theme {
    pub fn base(&self) -> Style {
        Style::default().fg(self.text).bg(self.background)
    }
}

/// Parses the theme presets file. Each color accepts `#RRGGBB`, an ANSI
/// color name, an ANSI index, or `reset`.
pub fn load(json: &str) -> Result<Vec<Theme>, String> {
    let root: Value =
        serde_json::from_str(json).map_err(|error| format!("Theme file is not JSON: {error}"))?;

    root.get("themes")
        .and_then(Value::as_array)
        .ok_or("Theme file has no themes list")?
        .iter()
        .map(parse_theme)
        .collect()
}

fn parse_theme(value: &Value) -> Result<Theme, String> {
    let field = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| format!("A theme has no {key}"))
    };
    let name = field("name")?;
    let appearance = field("appearance")?;

    let colors = value
        .get("colors")
        .ok_or_else(|| format!("{name}: colors are missing"))?;
    let color = |key: &str| {
        let raw = colors
            .get(key)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{name}: {key} is missing"))?;
        Color::from_str(raw).map_err(|_| format!("{name}: {key} has an invalid color {raw}"))
    };

    Ok(Theme {
        background: color("background")?,
        text: color("text")?,
        border: color("border")?,
        focus: color("focus")?,
        title: color("title")?,
        muted: color("muted")?,
        key: color("key")?,
        warning: color("warning")?,
        error: color("error")?,
        selection_text: color("selection_text")?,
        selection_background: color("selection_background")?,
        inactive_selection_background: color("inactive_selection_background")?,
        text_selection_text: color("text_selection_text")?,
        text_selection_background: color("text_selection_background")?,
        json_key: color("json_key")?,
        json_string: color("json_string")?,
        json_literal: color("json_literal")?,
        json_number: color("json_number")?,
        name: name.into(),
        appearance: appearance.into(),
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::config::{DEFAULT_THEME, THEMES_JSON};

    #[test]
    fn bundled_presets_parse_with_unique_names() {
        let themes = load(THEMES_JSON).unwrap();
        let names = themes
            .iter()
            .map(|theme| theme.name.as_ref())
            .chain([DEFAULT_THEME.name.as_ref()])
            .collect::<HashSet<_>>();

        assert_eq!(themes.len(), 20);
        assert_eq!(names.len(), 21);
        assert_eq!(themes[0].background, Color::Rgb(0x1e, 0x1e, 0x2e));
    }

    #[test]
    fn rejects_a_missing_or_invalid_color() {
        let missing = r#"{"themes":[{"name":"A","appearance":"dark","colors":{}}]}"#;
        let invalid = THEMES_JSON.replacen("#1e1e2e", "#zzzzzz", 1);

        assert_eq!(load(missing).unwrap_err(), "A: background is missing");
        assert!(
            load(&invalid)
                .unwrap_err()
                .contains("invalid color #zzzzzz")
        );
    }
}
