#![allow(clippy::unwrap_used, clippy::expect_used)]
use phantom_core::process_html;
use phantom_js::tier1::session::Tier1Session;
use phantom_serializer::{HeadlessSerializer, SerialiserConfig};
use std::time::Instant;

fn login_page_html() -> &'static str {
    r#"<!DOCTYPE html>
    <html lang="en">
    <head><meta charset="UTF-8"><title>Login</title></head>
    <body style="width: 1280px; height: 720px;">
        <header style="width: 1280px; height: 50px;"><nav><a href="/" style="width: 50px; height: 20px;">Home</a></nav></header>
        <main style="width: 400px; height: 400px;">
            <h1 style="width: 400px; height: 50px;">Sign In</h1>
            <form id="login-form" style="width: 400px; height: 300px;">
                <input id="email" type="email"
                       placeholder="Email" required aria-required="true" style="width: 200px; height: 30px;"/>
                <input id="password" type="password"
                       placeholder="Password" required aria-required="true" style="width: 200px; height: 30px;"/>
                <button id="submit" type="submit" data-testid="login-btn" style="width: 100px; height: 40px;">
                    Sign in
                </button>
                <a href="/forgot" style="width: 150px; height: 20px;">Forgot password?</a>
            </form>
        </main>
        <footer style="width: 1280px; height: 50px;"><p>Copyright 2026</p></footer>
    </body>
    </html>"#
}

#[tokio::test]
async fn phase2_full_pipeline_tier1() {
    // ═══════════════════════════════════════════════════════
    // STEP 1: Parse HTML with phantom-core
    // ═══════════════════════════════════════════════════════
    let start_total = Instant::now();

    let page = process_html(
        login_page_html(),
        "https://app.example.com/login",
        1280.0,
        720.0,
    )
    .expect("HTML parsing must not fail");

    assert!(
        page.tree.document_root.is_some(),
        "Document root must exist after parsing"
    );

    // ═══════════════════════════════════════════════════════
    // STEP 2: Create Tier 1 QuickJS session
    // ═══════════════════════════════════════════════════════
    let session_start = Instant::now();
    let mut session = Tier1Session::new()
        .await
        .expect("Tier1Session must create successfully");
    let session_elapsed = session_start.elapsed();

    println!("Tier1Session startup: {:?}", session_elapsed);
    assert!(
        session_elapsed.as_secs() < 5,
        "Session startup must complete in under 5 seconds"
    );

    // ═══════════════════════════════════════════════════════
    // STEP 3: Attach DOM to session
    // ═══════════════════════════════════════════════════════
    let dom_tree = page.tree.clone();
    session.attach_dom(dom_tree).await;

    // ═══════════════════════════════════════════════════════
    // STEP 4: Verify DOM bindings work from JS
    // ═══════════════════════════════════════════════════════

    // document.title must work
    let title = session
        .eval("document.title")
        .await
        .expect("document.title must not fail");
    println!("document.title = {}", title);
    assert_eq!(title, "Login", "document.title must return the page title");

    // querySelector must find the submit button
    let submit_type = session
        .eval("typeof document.querySelector('[data-testid=\"login-btn\"]')")
        .await
        .expect("querySelector must not fail");
    println!("submit button type = {}", submit_type);
    assert_eq!(
        submit_type, "object",
        "querySelector must find the submit button by data-testid"
    );

    // navigator.webdriver must be undefined (shim applied)
    let webdriver = session
        .eval("String(navigator.webdriver)")
        .await
        .expect("navigator.webdriver must not throw");
    assert_eq!(
        webdriver, "undefined",
        "navigator.webdriver must be undefined — shim must have applied"
    );

    // window.chrome must exist (shim applied)
    let chrome_type = session
        .eval("typeof window.chrome")
        .await
        .expect("window.chrome must not throw");
    assert_eq!(
        chrome_type, "object",
        "window.chrome must be an object — shim must have applied"
    );

    // ═══════════════════════════════════════════════════════
    // STEP 5: Serialise to CCT and verify output
    // ═══════════════════════════════════════════════════════
    let config = SerialiserConfig {
        url: "https://app.example.com/login".to_string(),
        viewport_width: 1280.0,
        viewport_height: 720.0,
        ..Default::default()
    };

    let cct = HeadlessSerializer::serialise(&page, &config);

    assert!(
        cct.starts_with("##PAGE"),
        "CCT must start with ##PAGE header"
    );
    assert!(
        cct.contains("url=https%3A%2F%2Fapp.example.com%2Flogin"),
        "CCT ##PAGE must contain the URL"
    );
    assert!(
        cct.contains("mode=full"),
        "CCT ##PAGE must contain mode=full"
    );

    let node_lines: Vec<&str> = cct
        .lines()
        .filter(|l| {
            l.chars()
                .next()
                .map(|c| c.is_ascii_alphanumeric())
                .unwrap_or(false)
                && l.contains('|')
        })
        .collect();

    println!("\n=== DIAGNOSTIC: FULL CCT ===");
    println!("{}", cct);
    println!("============================\n");

    assert!(
        !node_lines.is_empty(),
        "CCT must contain at least one node line"
    );

    // Verify CCT node format correctness
    if let Some(first_node) = node_lines.first() {
        let parts: Vec<&str> = first_node.split('|').collect();
        assert!(
            parts.len() >= 11,
            "CCT node must have at least 11 pipe-separated fields, got {}. Node: {}",
            parts.len(),
            first_node
        );
    }

    println!("\n=== PHASE 2 RESULTS ===");
    println!("Total pipeline time: {:?}", start_total.elapsed());
    println!("Session startup time: {:?}", session_elapsed);
    println!("CCT node count: {}", node_lines.len());
    println!("CCT size: {} bytes", cct.len());
    println!("First 3 node lines:");
    for line in node_lines.iter().take(3) {
        println!("  {}", line);
    }
    println!("Full CCT:\n{}", cct);

    // ═══════════════════════════════════════════════════════
    // STEP 6: Clean up
    // ═══════════════════════════════════════════════════════
    session.destroy();
}

#[test]
fn phase2_tier2_snapshot_in_pipeline() {
    // Verify Tier 2 session loads correctly and shims are pre-applied
    let mut session = phantom_js::tier2::session::Tier2Session::new(None)
        .expect("Tier2Session must load from snapshot");

    // Shims must be pre-applied from snapshot
    let webdriver = session.eval("String(navigator.webdriver)").unwrap();
    assert_eq!(
        webdriver, "undefined",
        "Tier2: navigator.webdriver shim must be pre-applied in snapshot"
    );

    let chrome = session.eval("typeof window.chrome").unwrap();
    assert_eq!(
        chrome, "object",
        "Tier2: window.chrome shim must be pre-applied in snapshot"
    );

    session.destroy();
}

#[tokio::test]
async fn phase2_behavior_engine_in_pipeline() {
    use phantom_js::BehaviorEngine;

    let engine = BehaviorEngine::new();

    // Verify a click sequence produces realistic timing
    let path = engine.generate_mouse_path((0.0, 0.0), (200.0, 150.0));
    assert!(
        path.len() >= 21,
        "Mouse path must have enough points, got {}",
        path.len()
    );

    let delay = engine.click_hesitation_ms();
    assert!(
        (20..=500).contains(&delay),
        "Click hesitation must be in valid range, got {}ms",
        delay
    );

    println!(
        "Phase 2 BehaviorEngine: path={} points, hesitation={}ms",
        path.len(),
        delay
    );
}
