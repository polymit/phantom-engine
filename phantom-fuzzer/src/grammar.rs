use rand::Rng;

use crate::config::ChaosLimits;
use crate::model::MutatorKind;

pub(crate) struct GeneratedDoc {
    pub doc: String,
    pub html: String,
    pub css: String,
    pub js: String,
    pub strategies: Vec<MutatorKind>,
    pub notes: Vec<String>,
}

pub(crate) fn build_doc<R: Rng>(
    rng: &mut R,
    limits: ChaosLimits,
    dom_depth: usize,
    max_css_rules: usize,
    max_js_depth: usize,
) -> GeneratedDoc {
    let html = build_dom(rng, dom_depth, 0);
    let css = build_css(rng, limits, max_css_rules);
    let js = build_js(rng, max_js_depth);
    let doc = format!(
        "<html><head><style>{css}</style></head><body>{html}<script>{js}</script></body></html>"
    );
    GeneratedDoc {
        doc,
        html,
        css,
        js,
        strategies: vec![MutatorKind::CssCascade, MutatorKind::JsGrammar],
        notes: vec![
            "cfg_generator".to_string(),
            "ax-tree differential check expected".to_string(),
        ],
    }
}

pub(crate) fn build_css<R: Rng>(rng: &mut R, limits: ChaosLimits, max_rules: usize) -> String {
    let mut out = String::from(":root{--a:var(--b);--b:var(--a);}");
    out.push_str(".chaos-z{position:relative;z-index:2147483648;width:calc(100% / 0);}");
    out.push_str(
        ".chaos-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(1px,1fr));}",
    );
    out.push_str(".chaos-flex{display:flex;}");
    let rule_count = rng.random_range(4..=max_rules.max(4));
    for idx in 0..rule_count {
        let selector = build_selector(rng, limits.max_selector_chain);
        let decls = build_decls(rng, idx);
        out.push_str(&selector);
        out.push('{');
        out.push_str(&decls);
        out.push('}');
    }
    out
}

pub(crate) fn build_js<R: Rng>(rng: &mut R, depth: usize) -> String {
    let gc_iters = rng.random_range(128..=1_024);
    let circle_depth = depth.max(2) * 8;
    format!(
        "function generate_circular_hell(depth){{let root={{}};let cur=root;for(let idx=0;idx<depth;idx++){{cur.next={{prev:cur}};cur=cur.next;}}cur.next=root;return root;}}
function trigger_gc_stress(){{for(let idx=0;idx<{gc_iters};idx++){{(function(){{let buf=new Uint8Array(10);return()=>buf;}})();}}}}
globalThis.__phantom_fuzz=generate_circular_hell({circle_depth});
trigger_gc_stress();"
    )
}

fn build_dom<R: Rng>(rng: &mut R, max_depth: usize, depth: usize) -> String {
    if depth >= max_depth {
        return leaf(rng);
    }
    let mut out = String::new();
    let count = rng.random_range(1..=3);
    for idx in 0..count {
        let tag = pick_tag(rng);
        let class = format!("class=\"c{} chaos-grid\"", rng.random_range(0..32));
        let id = format!("id=\"n{}-{}\"", depth, idx);
        let text = if rng.random_bool(0.35) {
            format!("txt-{}-{}", depth, idx)
        } else {
            String::new()
        };
        let child = build_dom(rng, max_depth, depth + 1);
        out.push('<');
        out.push_str(tag);
        out.push(' ');
        out.push_str(&class);
        out.push(' ');
        out.push_str(&id);
        out.push_str(" data-x=\"y\">");
        out.push_str(&text);
        out.push_str(&child);
        out.push_str("</");
        out.push_str(tag);
        out.push('>');
    }
    out
}

fn leaf<R: Rng>(rng: &mut R) -> String {
    let tag = pick_tag(rng);
    let text = format!("leaf-{}", rng.random_range(0..10_000));
    format!("<{tag} class=\"chaos-flex\">{text}</{tag}>")
}

fn pick_tag<R: Rng>(rng: &mut R) -> &'static str {
    const TAGS: &[&str] = &[
        "div", "span", "p", "a", "table", "canvas", "svg", "iframe", "video",
    ];
    let idx = rng.random_range(0..TAGS.len());
    TAGS[idx]
}

fn build_selector<R: Rng>(rng: &mut R, max_chain: usize) -> String {
    const BASE: &[&str] = &[
        "div",
        ".class",
        "#id",
        "p",
        "span",
        "a:hover",
        "[data-x=\"y\"]",
    ];
    const JOIN: &[&str] = &[" > ", " + ", " ~ "];
    let mut out = String::new();
    let count = rng.random_range(2..=max_chain.clamp(2, 6));
    for idx in 0..count {
        if idx > 0 {
            let op = JOIN[rng.random_range(0..JOIN.len())];
            out.push_str(op);
        }
        out.push_str(BASE[rng.random_range(0..BASE.len())]);
    }
    out.push_str(":nth-child(2n+1)");
    out
}

fn build_decls<R: Rng>(rng: &mut R, idx: usize) -> String {
    let width = match idx % 4 {
        0 => "-1px",
        1 => "1e10px",
        2 => "calc(100% / 0)",
        _ => "NaNpx",
    };
    let color = if rng.random_bool(0.5) {
        "rgba(0,0,0,0.000000000000000001)"
    } else {
        "url(javascript:alert(1))"
    };
    format!(
        "width:{width};display:inline-grid-flex;z-index:2147483648;color:{color};grid-template-columns:repeat(128,1fr);"
    )
}
