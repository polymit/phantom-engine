use super::properties::{ComputedStyle, Position};
use crate::dom::node::{Display, PointerEvents, SizeValue, Visibility};
use crate::dom::query::{PhantomParser, PhantomSelectorImpl};
use cssparser_selectors::{
    AtRuleParser, DeclarationParser as CssDeclarationParser, Parser, ParserInput,
    QualifiedRuleParser, RuleBodyParser, StyleSheetParser, ToCss, Token,
};
use selectors::SelectorList;

#[derive(Clone, Debug)]
pub struct CssRule {
    pub selectors: SelectorList<PhantomSelectorImpl>,
    pub declarations: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default)]
pub struct Stylesheet {
    pub rules: Vec<CssRule>,
}

impl Stylesheet {
    pub fn parse(css: &str) -> Self {
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        let mut rules = Vec::new();

        {
            let mut handler = StylesheetHandler { rules: &mut rules };
            let iter = StyleSheetParser::new(&mut parser, &mut handler);
            for result in iter {
                if let Err(e) = result {
                    tracing::debug!("css rule parse error: {:?}", e);
                }
            }
        }

        Self { rules }
    }
}

struct StylesheetHandler<'a> {
    rules: &'a mut Vec<CssRule>,
}

impl<'a, 'i> QualifiedRuleParser<'i> for StylesheetHandler<'a> {
    type Prelude = SelectorList<PhantomSelectorImpl>;
    type QualifiedRule = ();
    type Error = selectors::parser::SelectorParseErrorKind<'i>;

    fn parse_prelude<'t>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::Prelude, cssparser_selectors::ParseError<'i, Self::Error>> {
        SelectorList::parse(&PhantomParser, input, selectors::parser::ParseRelative::No)
    }

    fn parse_block<'t>(
        &mut self,
        prelude: Self::Prelude,
        _start: &cssparser_selectors::ParserState,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::QualifiedRule, cssparser_selectors::ParseError<'i, Self::Error>> {
        let mut declarations = Vec::new();
        let mut handler = DeclarationHandler {
            declarations: &mut declarations,
        };
        let iter = RuleBodyParser::new(input, &mut handler);
        for result in iter {
            if let Err(e) = result {
                tracing::debug!("css declaration parse error: {:?}", e);
            }
        }
        self.rules.push(CssRule {
            selectors: prelude,
            declarations,
        });
        Ok(())
    }
}

impl<'a, 'i> AtRuleParser<'i> for StylesheetHandler<'a> {
    type Prelude = ();
    type AtRule = ();
    type Error = selectors::parser::SelectorParseErrorKind<'i>;
}

struct DeclarationHandler<'a> {
    declarations: &'a mut Vec<(String, String)>,
}

impl<'a, 'i> CssDeclarationParser<'i> for DeclarationHandler<'a> {
    type Declaration = ();
    type Error = selectors::parser::SelectorParseErrorKind<'i>;

    fn parse_value<'t>(
        &mut self,
        name: cssparser_selectors::CowRcStr<'i>,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::Declaration, cssparser_selectors::ParseError<'i, Self::Error>> {
        let mut value = String::new();
        while let Ok(token) = input.next() {
            value.push_str(&token.to_css_string());
        }
        self.declarations
            .push((name.to_string(), value.trim().to_string()));
        Ok(())
    }
}

impl<'a, 'i> AtRuleParser<'i> for DeclarationHandler<'a> {
    type Prelude = ();
    type AtRule = ();
    type Error = selectors::parser::SelectorParseErrorKind<'i>;
}

impl<'a, 'i> QualifiedRuleParser<'i> for DeclarationHandler<'a> {
    type Prelude = ();
    type QualifiedRule = ();
    type Error = selectors::parser::SelectorParseErrorKind<'i>;

    fn parse_prelude<'t>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::Prelude, cssparser_selectors::ParseError<'i, Self::Error>> {
        Err(
            input.new_custom_error(selectors::parser::SelectorParseErrorKind::ClassNeedsIdent(
                cssparser_selectors::Token::Comma,
            )),
        )
    }

    fn parse_block<'t>(
        &mut self,
        _prelude: Self::Prelude,
        _start: &cssparser_selectors::ParserState,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::QualifiedRule, cssparser_selectors::ParseError<'i, Self::Error>> {
        Err(
            input.new_custom_error(selectors::parser::SelectorParseErrorKind::ClassNeedsIdent(
                cssparser_selectors::Token::Comma,
            )),
        )
    }
}

impl<'a, 'i>
    cssparser_selectors::RuleBodyItemParser<'i, (), selectors::parser::SelectorParseErrorKind<'i>>
    for DeclarationHandler<'a>
{
    fn parse_declarations(&self) -> bool {
        true
    }
    fn parse_qualified(&self) -> bool {
        false
    }
}

pub struct CssEngine;

impl CssEngine {
    fn parse_dimension(raw: &str) -> Option<SizeValue> {
        let val = raw.trim().to_lowercase();
        if val.is_empty() {
            return None;
        }

        if matches!(
            val.as_str(),
            "auto" | "min-content" | "max-content" | "fit-content" | "fit-content()"
        ) || val.starts_with("calc(")
        {
            return Some(SizeValue::Auto);
        }

        if let Some(num) = val.strip_suffix("px").and_then(|n| n.parse::<f32>().ok()) {
            return Some(SizeValue::Length(num));
        }
        if let Some(num) = val.strip_suffix('%').and_then(|n| n.parse::<f32>().ok()) {
            return Some(SizeValue::Percent(num / 100.0));
        }
        if let Some(num) = val.strip_suffix("vw").and_then(|n| n.parse::<f32>().ok()) {
            return Some(SizeValue::Percent(num / 100.0));
        }
        if let Some(num) = val.strip_suffix("vh").and_then(|n| n.parse::<f32>().ok()) {
            return Some(SizeValue::Percent(num / 100.0));
        }
        if let Some(num) = val.strip_suffix("rem").and_then(|n| n.parse::<f32>().ok()) {
            return Some(SizeValue::Length(num * 16.0));
        }
        if let Some(num) = val.strip_suffix("em").and_then(|n| n.parse::<f32>().ok()) {
            return Some(SizeValue::Length(num * 16.0));
        }
        if let Some(num) = val.strip_suffix("ch").and_then(|n| n.parse::<f32>().ok()) {
            return Some(SizeValue::Length(num * 8.0));
        }
        if let Some(num) = val.strip_suffix("fr").and_then(|n| n.parse::<f32>().ok()) {
            return Some(SizeValue::Percent(num));
        }
        val.parse::<f32>().ok().map(SizeValue::Length)
    }

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
                Token::Delim('-') if !in_value => {
                    current_prop.push('-');
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
        let val_clean = val.trim();
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
                style.visibility_set = true;
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
            "width" => {
                if let Some(dim) = Self::parse_dimension(val_clean) {
                    style.width = Some(dim);
                }
            }
            "height" => {
                if let Some(dim) = Self::parse_dimension(val_clean) {
                    style.height = Some(dim);
                }
            }
            _ => {}
        }
    }

    pub fn compute(
        inline_style: Option<&str>,
        parent_style: Option<&ComputedStyle>,
        stylesheet: Option<&Stylesheet>,
        node_id: indextree::NodeId,
        arena: &indextree::Arena<crate::dom::DomNode>,
    ) -> ComputedStyle {
        let mut style = ComputedStyle::default();

        // 1. Global Stylesheet Rules
        if let Some(sheet) = stylesheet {
            let el = crate::dom::query::DomElement { node_id, arena };
            let mut cache = selectors::NthIndexCache::default();
            let mut ctx = selectors::context::MatchingContext::new(
                selectors::context::MatchingMode::Normal,
                None,
                &mut cache,
                selectors::context::QuirksMode::NoQuirks,
                selectors::context::NeedsSelectorFlags::No,
                selectors::context::IgnoreNthChildForInvalidation::No,
            );

            for rule in &sheet.rules {
                if selectors::matching::matches_selector_list(&rule.selectors, &el, &mut ctx) {
                    for (prop, val) in &rule.declarations {
                        Self::apply_declaration(&mut style, prop, val);
                    }
                }
            }
        }

        // 2. Inline Style (Overrides Global)
        if let Some(inline) = inline_style {
            let inline_computed = Self::parse_inline_style(inline);
            // Merge inline into global (inline wins)
            if inline_computed.display != Display::Block {
                style.display = inline_computed.display;
            }
            if inline_computed.visibility_set {
                style.visibility = inline_computed.visibility;
                style.visibility_set = true;
            }
            style.opacity *= inline_computed.opacity;
            if let Some(w) = inline_computed.width {
                style.width = Some(w);
            }
            if let Some(h) = inline_computed.height {
                style.height = Some(h);
            }
            if let Some(z) = inline_computed.z_index {
                style.z_index = Some(z);
            }
            if inline_computed.pointer_events != PointerEvents::Auto {
                style.pointer_events = inline_computed.pointer_events;
            }
            if inline_computed.position != Position::Static {
                style.position = inline_computed.position;
            }
        }

        // 3. Inheritance
        if let Some(parent) = parent_style {
            // Visibility inherits only when child did not set visibility itself.
            if !style.visibility_set {
                style.visibility = parent.visibility.clone();
            }
            // Opacity always multiplies (child × parent)
            style.opacity *= parent.opacity; // Multiply by parent
        }

        style
    }
}

#[cfg(test)]
mod tests {
    use super::CssEngine;
    use crate::dom::node::{Display, PointerEvents, SizeValue, Visibility};

    #[test]
    fn parse_inline_style_keeps_last_declaration_without_trailing_semicolon() {
        let style = CssEngine::parse_inline_style("display: none; visibility: hidden");
        assert_eq!(style.display, Display::None);
        assert_eq!(style.visibility, Visibility::Hidden);
        assert!(style.visibility_set);
    }

    #[test]
    fn parse_inline_style_parses_hyphenated_last_property_without_semicolon() {
        let style = CssEngine::parse_inline_style("display: block; pointer-events: none");
        assert_eq!(style.display, Display::Block);
        assert_eq!(style.pointer_events, PointerEvents::None);
    }

    #[test]
    fn apply_declaration_does_not_silently_strip_value_semicolons() {
        let mut style = CssEngine::parse_inline_style("width: 10px");
        assert_eq!(style.width, Some(SizeValue::Length(10.0)));

        CssEngine::apply_declaration(&mut style, "width", "100px;");
        assert_eq!(style.width, Some(SizeValue::Length(10.0)));
    }

    #[test]
    fn parse_percent_and_relative_dimensions() {
        let width = CssEngine::parse_inline_style("width: 100%");
        assert_eq!(width.width, Some(SizeValue::Percent(1.0)));

        let rem = CssEngine::parse_inline_style("width: 2rem");
        assert_eq!(rem.width, Some(SizeValue::Length(32.0)));

        let vh = CssEngine::parse_inline_style("height: 50vh");
        assert_eq!(vh.height, Some(SizeValue::Percent(0.5)));
    }
}
