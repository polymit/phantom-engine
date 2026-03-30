use crate::dom::node::{Display, PointerEvents, Visibility};

#[derive(Debug, Clone, PartialEq)]
pub enum Position {
    Static,
    Relative,
    Absolute,
    Fixed,
    Sticky,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssValue {
    Keyword(String),
    Number(f32),
    Length(f32),
    Percentage(f32),
    None,
    Inherit,
    Initial,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComputedStyle {
    pub display: Display,
    pub visibility: Visibility,
    pub opacity: f32,
    pub position: Position,
    pub z_index: Option<i32>,
    pub pointer_events: PointerEvents,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            display: Display::Block,
            visibility: Visibility::Visible,
            opacity: 1.0,
            position: Position::Static,
            z_index: None,
            pointer_events: PointerEvents::Auto,
            width: None,
            height: None,
        }
    }
}
