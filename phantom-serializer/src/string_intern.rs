use crate::cct_types::{CctNode, BoundsConfidence};
use std::fmt::Write;

pub const CCT_BTN:  &str = "btn";
pub const CCT_INPT: &str = "inpt";
pub const CCT_DIV:  &str = "div";
pub const CCT_LNK:  &str = "lnk";
pub const CCT_FRM:  &str = "frm";
pub const CCT_SEL:  &str = "sel";
pub const CCT_TXT:  &str = "txt";
pub const CCT_CANV: &str = "canv";
pub const CCT_SVG:  &str = "svg";
pub const CCT_NAV:  &str = "nav";
pub const CCT_MAIN: &str = "main";
pub const CCT_HDR:  &str = "hdr";
pub const CCT_FTR:  &str = "ftr";
pub const CCT_IMG:  &str = "img";
pub const CCT_SPAN: &str = "span";

pub const PIPE: char = '|';

pub const DISP_B: char = 'b';
pub const DISP_N: char = 'n';
pub const DISP_I: char = 'i';
pub const DISP_F: char = 'f';
pub const DISP_G: char = 'g';

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
            self.events.to_string(),
            self.parent_id,
            self.flags,
            self.state.to_string(),
            self.id_confidence.to_char()
        );

        if let Some(r) = self.relevance {
            let _ = write!(buf, "|r:{}", r);
        }
    }
}
