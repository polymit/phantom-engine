#![allow(clippy::unwrap_used, clippy::expect_used)]
use phantom_core::dom::DomTree;
use phantom_js::tier1::session::Tier1Session;

/// Eval helper — unwraps for test convenience
async fn eval(s: &Tier1Session, code: &str) -> String {
    s.eval(code).await.expect("eval must not fail")
}

async fn eval_float(s: &Tier1Session, code: &str) -> f64 {
    let result = s.eval(code).await.expect("eval must not fail");
    result
        .parse::<f64>()
        .expect("result must be parseable as f64")
}

/// Stand up a Tier1Session with full shims loaded
async fn session_with_shims() -> Tier1Session {
    let mut session = Tier1Session::new().await.unwrap();
    let tree = DomTree::new();
    session.attach_dom(tree).await;

    // QuickJS doesn't have CanvasRenderingContext2D or document.fonts,
    // so we mock them. We then RE-EVALUATE the browser shims so that
    // the shim logic detects them and intercepts them.
    let mock_code = r#"
        globalThis.CanvasRenderingContext2D = function() {};
        CanvasRenderingContext2D.prototype.measureText = function(text) {
            // Fake default measureText behavior for un-shimmed fonts
            return { width: 50.0 };
        };
        
        globalThis.document = globalThis.document || {};
        document.fonts = {
            check: function(f, t) { return false; },
            load: function(f, t) { return Promise.reject("Not loaded"); }
        };
    "#;

    let eval_res = session.eval(&format!("(function(){{ try {{ {} }} catch(e) {{ return e.stack || e.toString(); }} return 'OK'; }})()", mock_code)).await;
    let res_str = eval_res.unwrap_or_else(|e| format!("{:?}", e));
    if res_str != "OK" {
        panic!("Mock error: {}", res_str);
    }

    // Re-apply ONLY Shim 14 over our mocks to avoid crashing on other shims (like navigator.plugins)
    // which are already marked configurable: false by the first evaluation in attach_dom()
    let shims_source = include_str!("../js/browser_shims.js");
    let start = shims_source
        .find("// 14. Font measureText")
        .expect("Could not find start of Shim 14");
    let end = shims_source
        .find("// 15. WebRTC IP leak prevention")
        .expect("Could not find start of Shim 15");
    let shim_14_code = &shims_source[start..end];

    let eval_res2 = session.eval(&format!("(function(){{ try {{ \n{}\n }} catch(e) {{ return String(e) + '\\n' + String(e.stack); }} return 'OK'; }})()", shim_14_code)).await;
    let res_str2 = eval_res2.unwrap_or_else(|e| format!("{:?}", e));
    if res_str2 != "OK" {
        panic!("Shim eval error: {}", res_str2);
    }

    session
}

#[tokio::test]
async fn font_shim_arial_width_is_correct() {
    let s = session_with_shims().await;
    let width = eval_float(
        &s,
        "
        (function() {
            var ctx = { font: '16px Arial' };
            return CanvasRenderingContext2D.prototype.measureText.call(ctx, 'x').width;
        })()
    ",
    )
    .await;
    assert!(
        (56.28..=56.32).contains(&width),
        "Arial at 16px must be ~56.30 (+-0.02), got {}",
        width
    );
    s.destroy();
}

#[tokio::test]
async fn font_shim_verdana_width_is_correct() {
    let s = session_with_shims().await;
    let width = eval_float(
        &s,
        "
        (function() {
            var ctx = { font: '16px Verdana' };
            return CanvasRenderingContext2D.prototype.measureText.call(ctx, 'x').width;
        })()
    ",
    )
    .await;
    assert!(
        (61.38..=61.42).contains(&width),
        "Verdana at 16px must be ~61.40, got {}",
        width
    );
    s.destroy();
}

#[tokio::test]
async fn font_shim_trebuchet_width_is_correct() {
    let s = session_with_shims().await;
    let width = eval_float(
        &s,
        "
        (function() {
            var ctx = { font: '16px \"Trebuchet MS\"' };
            return CanvasRenderingContext2D.prototype.measureText.call(ctx, 'x').width;
        })()
    ",
    )
    .await;
    assert!(
        (58.08..=58.12).contains(&width),
        "Trebuchet MS at 16px must be ~58.10, got {}",
        width
    );
    s.destroy();
}

#[tokio::test]
async fn font_shim_unknown_font_returns_number() {
    let s = session_with_shims().await;
    let t = eval(
        &s,
        "
        typeof CanvasRenderingContext2D.prototype.measureText.call(
            {font:'16px ZZZNonExistentFontXXX'}, 'x').width
    ",
    )
    .await;
    assert_eq!(
        t, "number",
        "unknown font must still return a number — not throw"
    );
    s.destroy();
}

#[tokio::test]
async fn font_shim_document_fonts_check_known() {
    let s = session_with_shims().await;
    let result = eval(
        &s,
        "String(document.fonts && document.fonts.check('16px Arial'))",
    )
    .await;
    assert_eq!(
        result, "true",
        "RISK-25: document.fonts.check must return true for known font"
    );
    s.destroy();
}

#[tokio::test]
async fn font_shim_document_fonts_load_known() {
    let s = session_with_shims().await;
    let result = eval(&s, "typeof document.fonts.load('16px Verdana').then").await;
    assert_eq!(
        result, "function",
        "RISK-25: document.fonts.load must return a Promise for known font"
    );
    s.destroy();
}

#[tokio::test]
async fn font_shim_impact_not_in_verdana_slot() {
    let s = session_with_shims().await;
    let impact = eval_float(
        &s,
        "
        (function() {
            var ctx = { font: '16px Impact' };
            return CanvasRenderingContext2D.prototype.measureText.call(ctx, 'x').width;
        })()
    ",
    )
    .await;

    let verdana = eval_float(
        &s,
        "
        (function() {
            var ctx = { font: '16px Verdana' };
            return CanvasRenderingContext2D.prototype.measureText.call(ctx, 'x').width;
        })()
    ",
    )
    .await;

    assert!((impact - 48.20).abs() < 0.05, "Impact must be ~48.20");
    assert!((verdana - 61.40).abs() < 0.05, "Verdana must be ~61.40");
    assert!(impact < verdana, "Impact must be narrower than Verdana");

    s.destroy();
}
