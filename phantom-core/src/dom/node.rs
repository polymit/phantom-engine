use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum NodeData {
    Document,
    Element {
        tag_name: String,
        attributes: HashMap<String, String>,
    },
    Text {
        content: String,
    },
    Comment {
        content: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Display {
    Block,
    None,
    Inline,
    Flex,
    Grid,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Visible,
    Hidden,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PointerEvents {
    Auto,
    None,
}

#[derive(Debug, Clone)]
pub enum AriaRole {
    Button,
    Link,
    Input,
    Navigation,
    Main,
    Header,
    Footer,
    Form,
    Dialog,
    Search,
    List,
    Table,
    Aside,
    None,
}

#[derive(Debug, Clone)]
pub enum EventListenerType {
    Click,
    Focus,
    Blur,
    Input,
    Submit,
    Keypress,
}

#[derive(Debug, Clone)]
pub struct DomNode {
    pub data: NodeData,
    pub is_visible: bool,
    pub computed_display: Display,
    pub computed_visibility: Visibility,
    pub computed_opacity: f32,
    pub computed_pointer_events: PointerEvents,
    pub computed_width: Option<f32>,
    pub computed_height: Option<f32>,
    pub z_index: Option<i32>,
    pub event_listeners: Vec<EventListenerType>,
    pub aria_role: Option<AriaRole>,
    pub aria_label: Option<String>,
}

impl DomNode {
    pub fn new(data: NodeData) -> Self {
        Self {
            data,
            is_visible: true,
            computed_display: Display::Block,
            computed_visibility: Visibility::Visible,
            computed_opacity: 1.0,
            computed_pointer_events: PointerEvents::Auto,
            computed_width: None,
            computed_height: None,
            z_index: None,
            event_listeners: Vec::new(),
            aria_role: None,
            aria_label: None,
        }
    }
}
