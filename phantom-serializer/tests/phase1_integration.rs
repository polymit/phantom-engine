#[cfg(test)]
mod phase1_integration {
    use phantom_core::process_html;
    use phantom_serializer::{HeadlessSerializer, SerialiserConfig, SerialiserMode};
    use std::time::Instant;

    fn login_page() -> &'static str {
        r#"<!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <title>Sign In — Example App</title>
        </head>
        <body>
            <header id="site-header">
                <nav aria-label="Main navigation">
                    <a href="/">Home</a>
                    <a href="/about">About</a>
                    <a href="/contact">Contact</a>
                </nav>
            </header>
            <main id="main-content">
                <h1>Sign In</h1>
                <form id="login-form" action="/login" method="post">
                    <div>
                        <label for="email">Email address</label>
                        <input
                            id="email"
                            type="email"
                            name="email"
                            placeholder="you@example.com"
                            required
                            aria-required="true"
                        />
                    </div>
                    <div>
                        <label for="password">Password</label>
                        <input
                            id="password"
                            type="password"
                            name="password"
                            placeholder="Your password"
                            required
                            aria-required="true"
                        />
                    </div>
                    <div style="display:none">
                        <input type="hidden" name="csrf" value="abc123"/>
                    </div>
                    <button
                        id="submit-btn"
                        type="submit"
                        data-testid="login-submit"
                    >
                        Sign in
                    </button>
                    <a href="/forgot-password">Forgot password?</a>
                </form>
            </main>
            <footer>
                <p>Copyright 2026 Example App</p>
            </footer>
        </body>
        </html>"#
    }

    #[test]
    fn phase1_full_pipeline() {
        let start_total = Instant::now();
        let page = process_html(
            login_page(),
            "https://app.example.com/login",
            1280.0,
            720.0,
        )
        .expect("HTML parsing must not fail");

        assert!(page.tree.document_root.is_some(), "Document root must exist");

        let submit = page.tree.get_element_by_id("submit-btn");
        assert!(submit.is_some(), "Submit button must be findable by ID");

        let email_input = page.tree.get_element_by_id("email");
        assert!(email_input.is_some(), "Email input must be findable by ID");

        let config = SerialiserConfig {
            url: "https://app.example.com/login".to_string(),
            viewport_width: 1280.0,
            viewport_height: 720.0,
            scroll_x: 0.0,
            scroll_y: 0.0,
            total_height: 720.0,
            mode: SerialiserMode::Full,
            task_hint: None,
        };

        let cct = HeadlessSerializer::serialise(&page, &config);
        let total_time = start_total.elapsed();

        assert!(cct.starts_with("##PAGE"), "CCT must start with ##PAGE header");
        assert!(
            cct.contains("url=https://app.example.com/login"),
            "CCT page header must contain URL"
        );
        assert!(cct.contains("viewport=1280x720"), "CCT page header must contain viewport");
        assert!(cct.contains("mode=full"), "CCT page header must contain mode");

        let node_lines: Vec<&str> = cct.lines().filter(|l| l.starts_with("n_")).collect();
        assert!(!node_lines.is_empty(), "CCT must contain at least one node line");

        println!("Full CCT output:\n{}", cct);

        println!("\n=== PHASE 1 PERFORMANCE ===");
        println!("Total time (parse + CSS + layout + serialise): {:?}", total_time);
        println!("Node count in CCT: {}", node_lines.len());
        println!("CCT size: {} bytes", cct.len());
        println!("First 3 node lines:");
        for line in node_lines.iter().take(3) {
            println!("  {}", line);
        }

        if let Some(first_node) = node_lines.first() {
            let parts: Vec<&str> = first_node.split('|').collect();
            assert!(
                parts.len() >= 11,
                "CCT node must have at least 11 pipe-separated fields, got {}",
                parts.len()
            );
            assert!(
                parts[0].starts_with("n_"),
                "Field 1 must be node ID starting with n_"
            );
            println!("First node fields count: {}", parts.len());
            println!("First node: {}", first_node);
        }
    }

    #[test]
    fn phase1_selective_mode_login() {
        let page = process_html(
            login_page(),
            "https://app.example.com/login",
            1280.0,
            720.0,
        )
        .unwrap();

        let config = SerialiserConfig {
            url: "https://app.example.com/login".to_string(),
            mode: SerialiserMode::Selective,
            task_hint: Some("find the login button and email input".to_string()),
            ..Default::default()
        };

        let selective_cct = HeadlessSerializer::serialise(&page, &config);
        let full_config = SerialiserConfig {
            url: "https://app.example.com/login".to_string(),
            ..Default::default()
        };
        let full_cct = HeadlessSerializer::serialise(&page, &full_config);

        let selective_nodes: usize = selective_cct.lines().filter(|l| l.starts_with("n_")).count();
        let full_nodes: usize = full_cct.lines().filter(|l| l.starts_with("n_")).count();

        println!("Full mode nodes:      {}", full_nodes);
        println!("Selective mode nodes: {}", selective_nodes);
        println!("Selective CCT:\n{}", selective_cct);

        assert!(selective_cct.contains("mode=selective"));
    }

    #[test]
    fn phase1_performance_benchmark() {
        fn make_large_page(divs: usize) -> String {
            let mut s = String::from("<!DOCTYPE html><html><body>");
            for i in 0..divs {
                s.push_str(&format!(
                    "<div class='card' id='card{}'>\
                        <h3>Card {}</h3>\
                        <p>Some content text here</p>\
                        <button data-testid='btn{}'>Action</button>\
                        <input type='text' placeholder='Field {}'/>
                    </div>",
                    i, i, i, i
                ));
            }
            s.push_str("</body></html>");
            s
        }

        // ~1000 DOM nodes (200 divs × ~5 nodes each)
        let html = make_large_page(200);
        let page = process_html(&html, "https://bench.test", 1280.0, 720.0).unwrap();

        let config = SerialiserConfig {
            url: "https://bench.test".to_string(),
            ..Default::default()
        };

        // Warm-up: initialises the global buffer pool
        let _ = HeadlessSerializer::serialise(&page, &config);

        let iterations = 10u32;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = HeadlessSerializer::serialise(&page, &config);
        }
        let elapsed = start.elapsed();
        let avg_us = elapsed.as_micros() / iterations as u128;
        let avg_ms = avg_us as f64 / 1000.0;

        println!("\n=== PHASE 1 BENCHMARK ===");
        println!("Iterations: {}", iterations);
        println!("Average time: {:.2}ms ({} µs)", avg_ms, avg_us);
        println!("Target: <10ms (minimum), <5ms (goal)");
        println!(
            "Status: {}",
            if avg_ms < 5.0 {
                "✅ GOAL MET"
            } else if avg_ms < 10.0 {
                "⚠️  MINIMUM MET (goal not yet reached in debug build)"
            } else {
                "❌ BELOW MINIMUM — optimisation required"
            }
        );
        // Timing is machine-dependent; HQ reviews the printed value.
    }
}
