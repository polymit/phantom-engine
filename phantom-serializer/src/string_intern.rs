use crate::cct_types::{BoundsConfidence, CctDelta, CctNode};
use std::fmt::{self, Write};

impl CctNode {
    pub fn serialise_into(&self, buf: &mut String) {
        let t_code = self.element_type.to_cct_code();
        let r_code = self.aria_role.to_cct_code();
        
        let mut s_acc = self.accessible_name.as_str();
        if s_acc.is_empty() { s_acc = "-"; }
        else if s_acc.chars().count() > 100 {
            if let Some((idx, _)) = s_acc.char_indices().nth(100) {
                s_acc = &s_acc[..idx];
            }
        }

        let mut s_vis = self.visible_text.as_str();
        if s_vis.is_empty() { s_vis = "-"; }
        else if s_vis.chars().count() > 100 {
            if let Some((idx, _)) = s_vis.char_indices().nth(100) {
                s_vis = &s_vis[..idx];
            }
        }

        let b_unrel = match self.bounds_confidence {
            BoundsConfidence::Reliable => "",
            BoundsConfidence::Unreliable => "~",
        };

        let _ = write!(
            buf,
            "{}|{}|{}|{},{},{},{}{}|{},{},{:.1},{}|{}|{}|{}|{}|{}|{}|{}",
            self.node_id,
            t_code,
            r_code,
            self.x, self.y, self.w, self.h, b_unrel,
            self.display.to_char(),
            self.visibility.to_char(),
            self.opacity,
            self.pointer_events.to_char(),
            s_acc,
            s_vis,
            self.events,
            self.parent_id,
            self.flags,
            self.state,
            self.id_confidence.to_char()
        );

        if let Some(r) = self.relevance {
            let _ = write!(buf, "|r:{}", r);
        }
    }
}

impl fmt::Display for CctDelta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Add(node_id) => write!(f, "+ {}", node_id),
            Self::Remove(node_id) => write!(f, "- {}", node_id),
            Self::Update { node_id, display, bounds } => {
                write!(f, "~ {}", node_id)?;
                match display {
                    Some(d) => write!(f, "|{}", d.to_char())?,
                    None => write!(f, "|-")?,
                }
                if let Some((x, y, w, h)) = bounds {
                    write!(f, "|{},{},{},{}", x, y, w, h)?;
                }
                Ok(())
            }
            Self::Scroll { x, y } => write!(f, "##SCROLL {},{}", x, y),
        }
    }
}
