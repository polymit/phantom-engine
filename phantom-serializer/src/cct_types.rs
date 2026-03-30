use phantom_core::dom::{AriaRole, Display, EventListenerType, PointerEvents, Visibility};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementType {
    Btn,
    Inpt,
    Div,
    Lnk,
    Frm,
    Sel,
    Txt,
    Canv,
    Svg,
    Nav,
    Main,
    Hdr,
    Ftr,
    Img,
    Span,
    Li,
    Ul,
    Tbl,
    Other(String),
}

impl ElementType {
    pub fn from_tag(tag: &str) -> Self {
        match tag.to_lowercase().as_str() {
            "button" => Self::Btn,
            "input" | "textarea" => Self::Inpt,
            "div" => Self::Div,
            "a" => Self::Lnk,
            "form" => Self::Frm,
            "select" => Self::Sel,
            "canvas" => Self::Canv,
            "svg" => Self::Svg,
            "nav" => Self::Nav,
            "main" => Self::Main,
            "header" => Self::Hdr,
            "footer" => Self::Ftr,
            "img" => Self::Img,
            "span" => Self::Span,
            "li" => Self::Li,
            "ul" | "ol" => Self::Ul,
            "table" => Self::Tbl,
            other => Self::Other(other.to_string()),
        }
    }

    pub fn to_cct_code(&self) -> &str {
        match self {
            Self::Btn => "btn",
            Self::Inpt => "inpt",
            Self::Div => "div",
            Self::Lnk => "lnk",
            Self::Frm => "frm",
            Self::Sel => "sel",
            Self::Txt => "txt",
            Self::Canv => "canv",
            Self::Svg => "svg",
            Self::Nav => "nav",
            Self::Main => "main",
            Self::Hdr => "hdr",
            Self::Ftr => "ftr",
            Self::Img => "img",
            Self::Span => "spn",
            Self::Li => "li",
            Self::Ul => "ul",
            Self::Tbl => "tbl",
            Self::Other(_) => "oth",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CctAriaRole {
    Btn,
    Lnk,
    Ipt,
    Sel,
    Nav,
    Main,
    Hdr,
    Ftr,
    Frm,
    Dlg,
    Src,
    Lst,
    Tbl,
    Asd,
    None,
}

impl CctAriaRole {
    pub fn from_aria_role(role: &Option<AriaRole>) -> Self {
        match role {
            Some(AriaRole::Button) => Self::Btn,
            Some(AriaRole::Link) => Self::Lnk,
            Some(AriaRole::Input) => Self::Ipt,
            Some(AriaRole::Navigation) => Self::Nav,
            Some(AriaRole::Main) => Self::Main,
            Some(AriaRole::Header) => Self::Hdr,
            Some(AriaRole::Footer) => Self::Ftr,
            Some(AriaRole::Form) => Self::Frm,
            Some(AriaRole::Dialog) => Self::Dlg,
            Some(AriaRole::Search) => Self::Src,
            Some(AriaRole::List) => Self::Lst,
            Some(AriaRole::Table) => Self::Tbl,
            Some(AriaRole::Aside) => Self::Asd,
            _ => Self::None,
        }
    }

    pub fn from_tag(tag: &str) -> Self {
        match tag.to_lowercase().as_str() {
            "button" => Self::Btn,
            "a" => Self::Lnk,
            "input" | "textarea" => Self::Ipt,
            "select" => Self::Sel,
            "nav" => Self::Nav,
            "main" => Self::Main,
            "header" => Self::Hdr,
            "footer" => Self::Ftr,
            "form" => Self::Frm,
            "dialog" => Self::Dlg,
            "search" => Self::Src,
            "ul" | "ol" => Self::Lst,
            "table" => Self::Tbl,
            "aside" => Self::Asd,
            _ => Self::None,
        }
    }

    pub fn to_cct_code(&self) -> &str {
        match self {
            Self::Btn => "btn",
            Self::Lnk => "lnk",
            Self::Ipt => "ipt",
            Self::Sel => "sel",
            Self::Nav => "nav",
            Self::Main => "main",
            Self::Hdr => "hdr",
            Self::Ftr => "ftr",
            Self::Frm => "frm",
            Self::Dlg => "dlg",
            Self::Src => "src",
            Self::Lst => "lst",
            Self::Tbl => "tbl",
            Self::Asd => "asd",
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CctDisplay {
    B,
    N,
    I,
    F,
    G,
}

impl CctDisplay {
    pub fn from_display(d: &Display) -> Self {
        match d {
            Display::Block => Self::B,
            Display::None => Self::N,
            Display::Inline => Self::I,
            Display::Flex => Self::F,
            Display::Grid => Self::G,
        }
    }

    pub fn to_char(&self) -> char {
        match self {
            Self::B => 'b',
            Self::N => 'n',
            Self::I => 'i',
            Self::F => 'f',
            Self::G => 'g',
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CctVisibility {
    V,
    H,
}

impl CctVisibility {
    pub fn from_visibility(v: &Visibility) -> Self {
        match v {
            Visibility::Visible => Self::V,
            Visibility::Hidden => Self::H,
        }
    }

    pub fn to_char(&self) -> char {
        match self {
            Self::V => 'v',
            Self::H => 'h',
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CctPointerEvents {
    A,
    N,
}

impl CctPointerEvents {
    pub fn from_pointer_events(pe: &PointerEvents) -> Self {
        match pe {
            PointerEvents::Auto => Self::A,
            PointerEvents::None => Self::N,
        }
    }

    pub fn to_char(&self) -> char {
        match self {
            Self::A => 'a',
            Self::N => 'n',
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CctState {
    pub disabled: bool,
    pub checked: bool,
    pub selected: bool,
    pub expanded: bool,
    pub required: bool,
    pub loading: bool,
    pub readonly: bool,
    pub error: bool,
    pub focused: bool,
    pub busy: bool,
    pub invalid: bool,
}

impl CctState {
    pub fn empty() -> Self {
        Self {
            disabled: false,
            checked: false,
            selected: false,
            expanded: false,
            required: false,
            loading: false,
            readonly: false,
            error: false,
            focused: false,
            busy: false,
            invalid: false,
        }
    }

    pub fn from_attributes(attrs: &HashMap<String, String>) -> Self {
        let mut state = Self::empty();
        
        let d = attrs.get("disabled").is_some() || attrs.get("aria-disabled").is_some_and(|v| v == "true");
        let ro = attrs.get("readonly").is_some() || attrs.get("aria-readonly").is_some_and(|v| v == "true");
        let ch = attrs.get("checked").is_some() || attrs.get("aria-checked").is_some_and(|v| v == "true");
        let sel = attrs.get("aria-selected").is_some_and(|v| v == "true");
        let exp = attrs.get("aria-expanded").is_some_and(|v| v == "true");
        let req = attrs.get("required").is_some() || attrs.get("aria-required").is_some_and(|v| v == "true");
        let bsy = attrs.get("aria-busy").is_some_and(|v| v == "true");
        let inv = attrs.get("aria-invalid").is_some_and(|v| v == "true" || v == "spelling" || v == "grammar");
        
        state.disabled = d;
        state.checked = ch;
        state.selected = sel;
        state.expanded = exp;
        state.required = req;
        state.readonly = ro;
        state.busy = bsy;
        state.invalid = inv;
        
        state
    }


}

impl fmt::Display for CctState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "s:{},{},{},{},{},{},{},{},{},{},{}",
            self.disabled as u8, self.checked as u8, self.selected as u8,
            self.expanded as u8, self.required as u8, self.loading as u8,
            self.readonly as u8, self.error as u8, self.focused as u8,
            self.busy as u8, self.invalid as u8,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdConfidence {
    High,
    Medium,
    Low,
}

impl IdConfidence {
    pub fn to_char(&self) -> char {
        match self {
            Self::High => 'h',
            Self::Medium => 'm',
            Self::Low => 'l',
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CctEvents {
    pub click: bool,
    pub focus: bool,
    pub blur: bool,
    pub input: bool,
    pub submit: bool,
    pub keypress: bool,
}

impl CctEvents {
    pub fn empty() -> Self {
        Self {
            click: false,
            focus: false,
            blur: false,
            input: false,
            submit: false,
            keypress: false,
        }
    }

    pub fn from_event_listeners(listeners: &[EventListenerType]) -> Self {
        let mut evts = Self::empty();
        for l in listeners {
            match l {
                EventListenerType::Click => evts.click = true,
                EventListenerType::Focus => evts.focus = true,
                EventListenerType::Blur => evts.blur = true,
                EventListenerType::Input => evts.input = true,
                EventListenerType::Submit => evts.submit = true,
                EventListenerType::Keypress => evts.keypress = true,
            }
        }
        evts
    }


}

impl fmt::Display for CctEvents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = String::with_capacity(12);
        if self.click   { buf.push_str("c,"); }
        if self.focus   { buf.push_str("f,"); }
        if self.blur    { buf.push_str("b,"); }
        if self.input   { buf.push_str("i,"); }
        if self.submit  { buf.push_str("s,"); }
        if self.keypress{ buf.push_str("k,"); }
        if buf.is_empty() {
            f.write_str("-")
        } else {
            // Trim trailing comma added above for convenience
            f.write_str(buf.trim_end_matches(','))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundsConfidence {
    Reliable,
    Unreliable,
}

#[derive(Debug, Clone)]
pub struct CctNode {
    pub node_id: String,
    pub element_type: ElementType,
    pub aria_role: CctAriaRole,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub bounds_confidence: BoundsConfidence,
    pub display: CctDisplay,
    pub visibility: CctVisibility,
    pub opacity: f32,
    pub pointer_events: CctPointerEvents,
    pub accessible_name: String,
    pub visible_text: String,
    pub events: CctEvents,
    pub parent_id: String,
    pub flags: u8,
    pub state: CctState,
    pub id_confidence: IdConfidence,
    pub relevance: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerialiserMode {
    Full,
    Selective,
}

pub struct CctPageHeader {
    pub url: String,
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub total_width: f32,
    pub total_height: f32,
    pub node_count: usize,
    pub mode: SerialiserMode,
}

impl fmt::Display for CctPageHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let m = match self.mode {
            SerialiserMode::Full => "full",
            SerialiserMode::Selective => "selective",
        };
        write!(
            f,
            "##PAGE url={} scroll={},{} viewport={}x{} total={},{} nodes={} mode={}",
            self.url,
            self.scroll_x, self.scroll_y,
            self.viewport_width, self.viewport_height,
            self.total_width, self.total_height,
            self.node_count, m
        )
    }
}

pub enum CctDelta {
    Add(indextree::NodeId),
    Remove(String),
    Update {
        node_id: String,
        display: Option<CctDisplay>,
        bounds: Option<(f32, f32, f32, f32)>,
    },
    Scroll {
        x: f32,
        y: f32,
    },
}

pub enum LandmarkType {
    Nav,
    Main,
    Form,
    Dialog,
    Search,
    List,
    Table,
    Header,
    Footer,
    Aside,
}

impl LandmarkType {
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag.to_lowercase().as_str() {
            "nav" => Some(Self::Nav),
            "main" => Some(Self::Main),
            "form" => Some(Self::Form),
            "dialog" => Some(Self::Dialog),
            "search" => Some(Self::Search),
            "ul" | "ol" => Some(Self::List),
            "table" => Some(Self::Table),
            "header" => Some(Self::Header),
            "footer" => Some(Self::Footer),
            "aside" => Some(Self::Aside),
            _ => None,
        }
    }

    pub fn from_aria_role(role: &str) -> Option<Self> {
        match role.to_lowercase().as_str() {
            "navigation" => Some(Self::Nav),
            "main" => Some(Self::Main),
            "form" => Some(Self::Form),
            "dialog" | "alertdialog" => Some(Self::Dialog),
            "search" => Some(Self::Search),
            "list" => Some(Self::List),
            "table" | "grid" | "treegrid" => Some(Self::Table),
            "banner" => Some(Self::Header),
            "contentinfo" => Some(Self::Footer),
            "complementary" => Some(Self::Aside),
            _ => None,
        }
    }

    pub fn to_marker(&self, node_id: &str) -> String {
        let m = match self {
            Self::Nav => "NAV",
            Self::Main => "MAIN",
            Self::Form => "FORM",
            Self::Dialog => "DIALOG",
            Self::Search => "SEARCH",
            Self::List => "LIST",
            Self::Table => "TABLE",
            Self::Header => "HEADER",
            Self::Footer => "FOOTER",
            Self::Aside => "ASIDE",
        };
        format!("##{} {}", m, node_id)
    }
}
