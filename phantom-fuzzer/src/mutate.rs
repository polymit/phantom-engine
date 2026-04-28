use std::collections::HashMap;

use phantom_core::dom::{DomNode, NodeData};
use phantom_core::{parse_html, DomTree, NodeId};
use rand::Rng;

use crate::config::ChaosLimits;
use crate::error::Result;
use crate::grammar::{build_css, build_js};
use crate::model::{MutatorKind, Seed};
use crate::render::render_doc;

pub(crate) struct MutatedDoc {
    pub doc: String,
    pub html: String,
    pub css: String,
    pub js: String,
    pub strategies: Vec<MutatorKind>,
    pub notes: Vec<String>,
}

pub(crate) fn mutate_seed<R: Rng>(
    rng: &mut R,
    seed: &Seed,
    limits: ChaosLimits,
    max_css_rules: usize,
    max_js_depth: usize,
) -> Result<MutatedDoc> {
    let mut tree = parse_html(&seed.html);
    let mut notes = Vec::new();
    let mut strategies = Vec::new();

    wrap_target(rng, &mut tree, limits, &mut notes, &mut strategies);
    bloat_target(rng, &mut tree, limits, &mut notes, &mut strategies);
    lace_text(
        rng,
        &mut tree,
        limits.zero_width_hits,
        &mut notes,
        &mut strategies,
    );

    let css = build_css(rng, limits, max_css_rules);
    let js = build_js(rng, max_js_depth);
    let html = render_doc(&tree)?;
    let html = splice_tag(rng, &html, &mut notes, &mut strategies);
    let doc = inject_assets(&html, &css, &js);

    Ok(MutatedDoc {
        doc,
        html,
        css,
        js,
        strategies,
        notes,
    })
}

fn wrap_target<R: Rng>(
    rng: &mut R,
    tree: &mut DomTree,
    limits: ChaosLimits,
    notes: &mut Vec<String>,
    strategies: &mut Vec<MutatorKind>,
) {
    let Some(target) = pick_target(rng, tree) else {
        return;
    };
    let wraps = rng.random_range(limits.nesting_min..=limits.nesting_max);
    for _ in 0..wraps {
        let wrapper = tree.arena.new_node(DomNode::new(NodeData::Element {
            tag_name: "div".to_string(),
            attributes: HashMap::from([("class".to_string(), "fuzz-wrap".to_string())]),
        }));
        target.insert_before(wrapper, &mut tree.arena);
        target.detach(&mut tree.arena);
        wrapper.append(target, &mut tree.arena);
    }
    notes.push(format!("wrapped one node {wraps} times"));
    strategies.push(MutatorKind::InfiniteNesting);
}

fn bloat_target<R: Rng>(
    rng: &mut R,
    tree: &mut DomTree,
    limits: ChaosLimits,
    notes: &mut Vec<String>,
    strategies: &mut Vec<MutatorKind>,
) {
    let Some(target) = pick_target(rng, tree) else {
        return;
    };
    let Some(NodeData::Element { attributes, .. }) =
        tree.get_mut(target).map(|node| &mut node.data)
    else {
        return;
    };

    for idx in 0..limits.attr_bloat {
        let key = if idx == 0 {
            "data-chaos-root".to_string()
        } else {
            format!("data-chaos-{idx}")
        };
        attributes.insert(key, format!("v{idx}"));
    }

    if let Some(val) = attributes.remove("data-chaos-root") {
        attributes.insert("data\u{200B}-chaos-root".to_string(), val);
    }

    notes.push(format!("added {} data-* attrs", limits.attr_bloat));
    strategies.push(MutatorKind::AttributeBloat);
    strategies.push(MutatorKind::ZeroWidth);
}

fn lace_text<R: Rng>(
    rng: &mut R,
    tree: &mut DomTree,
    hits: usize,
    notes: &mut Vec<String>,
    strategies: &mut Vec<MutatorKind>,
) {
    if hits == 0 {
        return;
    }
    let text_nodes: Vec<_> = tree
        .document_root
        .into_iter()
        .flat_map(|root| root.descendants(&tree.arena))
        .filter(|node_id| {
            matches!(
                tree.get(*node_id).map(|node| &node.data),
                Some(NodeData::Text { .. })
            )
        })
        .collect();

    if text_nodes.is_empty() {
        return;
    }

    let target = text_nodes[rng.random_range(0..text_nodes.len())];
    let Some(NodeData::Text { content }) = tree.get_mut(target).map(|node| &mut node.data) else {
        return;
    };
    let mut out = String::new();
    let chars: Vec<_> = content.chars().collect();
    for (idx, ch) in chars.iter().enumerate() {
        out.push(*ch);
        if idx < hits.min(chars.len()) {
            out.push('\u{200B}');
        }
    }
    *content = out;
    notes.push(format!(
        "inserted {} zero-width text hits",
        hits.min(chars.len())
    ));
    if !strategies
        .iter()
        .any(|kind| matches!(kind, MutatorKind::ZeroWidth))
    {
        strategies.push(MutatorKind::ZeroWidth);
    }
}

fn pick_target<R: Rng>(rng: &mut R, tree: &DomTree) -> Option<NodeId> {
    let nodes: Vec<_> = tree
        .document_root
        .into_iter()
        .flat_map(|root| root.descendants(&tree.arena))
        .filter(|node_id| {
            matches!(
                tree.get(*node_id).map(|node| &node.data),
                Some(NodeData::Element { .. })
            ) && tree
                .arena
                .get(*node_id)
                .and_then(|node| node.parent())
                .is_some()
        })
        .collect();

    if nodes.is_empty() {
        return None;
    }

    Some(nodes[rng.random_range(0..nodes.len())])
}

fn splice_tag<R: Rng>(
    rng: &mut R,
    html: &str,
    notes: &mut Vec<String>,
    strategies: &mut Vec<MutatorKind>,
) -> String {
    const FROM: &[&str] = &["</p>", "</span>", "</div>", "</a>"];
    const TO: &[&str] = &["</table>", "</tbody>", "</svg>"];

    for tag in FROM {
        if let Some(idx) = html.find(tag) {
            let mut out = html.to_string();
            let repl = TO[rng.random_range(0..TO.len())];
            out.replace_range(idx..idx + tag.len(), repl);
            notes.push(format!("spliced closing tag {tag} -> {repl}"));
            strategies.push(MutatorKind::TagSplice);
            return out;
        }
    }

    html.to_string()
}

fn inject_assets(html: &str, css: &str, js: &str) -> String {
    let style = format!("<style>{css}</style>");
    let script = format!("<script>{js}</script>");

    if html.contains("</head>") {
        let html = html.replacen("</head>", &format!("{style}</head>"), 1);
        if html.contains("</body>") {
            return html.replacen("</body>", &format!("{script}</body>"), 1);
        }
        return format!("{html}{script}");
    }

    if html.contains("</body>") {
        return html.replacen("</body>", &format!("{style}{script}</body>"), 1);
    }

    format!("<html><head>{style}</head><body>{html}{script}</body></html>")
}
