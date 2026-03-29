use super::properties::{ComputedStyle, Position};
use crate::dom::node::{Display, PointerEvents, Visibility};
use cssparser::{Parser, ParserInput, ToCss, Token};

pub struct CssEngine;

impl CssEngine {
    pub fn parse_inline_style(style_attr: &str) -> ComputedStyle {
        let mut style = ComputedStyle::default();
        let mut input = ParserInput::new(style_attr);
        let mut parser = Parser::new(&mut input);

        let mut current_prop = String::new();
        let mut current_val = String::new();
        let mut in_value = false;

        while let Ok(token) = parser.next() {
            match token {
                Token::Colon if !in_value => {
                    in_value = true;
                }
                Token::Semicolon => {
                    if in_value && !current_prop.is_empty() {
                        Self::apply_declaration(&mut style, &current_prop, current_val.trim());
                    }
                    current_prop.clear();
                    current_val.clear();
                    in_value = false;
                }
                Token::Ident(ref name) if !in_value => {
                    current_prop.push_str(name);
                }
                Token::WhiteSpace(_) => {
                    if in_value {
                        current_val.push(' ');
                    }
                }
                t => {
                    if in_value {
                        current_val.push_str(&t.to_css_string());
                    }
                }
            }
        }

        if in_value && !current_prop.is_empty() {
            Self::apply_declaration(&mut style, &current_prop, current_val.trim());
        }

        style
    }

    pub fn apply_declaration(style: &mut ComputedStyle, property: &str, value: &str) {
        let val = value.to_lowercase();
        // Just take the first token string, discarding the semicolon
        let val_clean = val.trim_end_matches(';').trim();
        match property.to_lowercase().as_str() {
            "display" => {
                style.display = match val_clean {
                    "none" => Display::None,
                    "inline" => Display::Inline,
                    "flex" => Display::Flex,
                    "grid" => Display::Grid,
                    _ => Display::Block,
                };
            }
            "visibility" => {
                style.visibility = match val_clean {
                    "hidden" => Visibility::Hidden,
                    _ => Visibility::Visible,
                };
            }
            "opacity" => {
                if let Ok(v) = val_clean.parse::<f32>() {
                    style.opacity = v.clamp(0.0, 1.0);
                } else {
                    style.opacity = 1.0;
                }
            }
            "position" => {
                style.position = match val_clean {
                    "relative" => Position::Relative,
                    "absolute" => Position::Absolute,
                    "fixed" => Position::Fixed,
                    "sticky" => Position::Sticky,
                    _ => Position::Static,
                };
            }
            "z-index" => {
                style.z_index = if val_clean == "auto" {
                    None
                } else {
                    val_clean.parse::<i32>().ok()
                };
            }
            "pointer-events" => {
                style.pointer_events = match val_clean {
                    "none" => PointerEvents::None,
                    _ => PointerEvents::Auto,
                };
            }
            _ => {}
        }
    }

    pub fn compute(
        inline_style: Option<&str>,
        parent_style: Option<&ComputedStyle>,
    ) -> ComputedStyle {
        let mut style = if let Some(inline) = inline_style {
            Self::parse_inline_style(inline)
        } else {
            ComputedStyle::default()
        };

        if let Some(parent) = parent_style {
            style.visibility = parent.visibility.clone();
            style.opacity *= parent.opacity; // Multiply by parent
        }

        style
    }
}
