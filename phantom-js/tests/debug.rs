#![allow(clippy::unwrap_used, clippy::expect_used)]
use phantom_core::dom::DomTree;
use phantom_js::tier1::session::Tier1Session;
#[tokio::main]
async fn main() {
    let mut session = Tier1Session::new().await.unwrap();
    let tree = DomTree::new();
    session.attach_dom(tree).await;
    let res = session
        .eval(
            r#"
        globalThis.CanvasRenderingContext2D = function() {};
        CanvasRenderingContext2D.prototype.measureText = function(text) {
            return { width: 50.0 };
        };
        globalThis.document = globalThis.document || {};
        document.fonts = {
            check: function(f, t) { return false; },
            load: function(f, t) { return Promise.reject("Not loaded"); }
        };
    "#,
        )
        .await;
    println!("MOCK Eval Result: {:?}", res);

    let shims_source = include_str!("../js/browser_shims.js");
    let res = session.eval(shims_source).await;
    println!("SHIM Eval Result: {:?}", res);
}
