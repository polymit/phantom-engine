use crate::cct_types::{BoundsConfidence, CctDelta, CctNode};
use std::fmt::{self, Write};

fn clip_100(s: &str) -> &str {
    if s.chars().count() <= 100 {
        return s;
    }
    let end = s.char_indices().nth(100).map(|(i, _)| i).unwrap_or(s.len());
    &s[..end]
}

fn encode_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '%' => out.push_str("%25"),
            '|' => out.push_str("%7C"),
            ',' => out.push_str("%2C"),
            '\n' => out.push_str("%0A"),
            '\r' => out.push_str("%0D"),
            _ => out.push(ch),
        }
    }
    out
}

impl CctNode {
    pub fn serialise_into(&self, buf: &mut String) {
        let t_code = self.element_type.to_cct_code();
        let r_code = self.aria_role.to_cct_code();

        let acc_raw = if self.accessible_name.is_empty() {
            "-"
        } else {
            clip_100(self.accessible_name.as_str())
        };
        let vis_raw = if self.visible_text.is_empty() {
            "-"
        } else {
            clip_100(self.visible_text.as_str())
        };
        let s_acc = encode_text(acc_raw);
        let s_vis = encode_text(vis_raw);

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
            self.x,
            self.y,
            self.w,
            self.h,
            b_unrel,
            self.display.to_char(),
            self.visibility.to_char(),
            self.opacity,
            self.pointer_events.to_char(),
            s_acc.as_str(),
            s_vis.as_str(),
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
            Self::Update {
                node_id,
                display,
                bounds,
            } => {
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
