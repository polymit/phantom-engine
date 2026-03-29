#[cfg(test)]
mod tests {
    use phantom_core::process_html;
    use phantom_serializer::{HeadlessSerializer, SerialiserConfig, SerialiserMode};
    use std::time::Instant;

    fn make_page(n: usize) -> String {
        let mut html = String::from("<html><body>");
        for i in 0..n {
            html.push_str(&format!(
                "<div id='node{}'><button>Btn {}</button>\
                 <input type='text' placeholder='Field {}'/>\
                 </div>", i, i, i
            ));
        }
        html.push_str("</body></html>");
        html
    }

    #[test]
    fn test_serialise_basic_page() {
        let page = process_html(
            r#"<html><body>
                <nav><a href="/">Home</a></nav>
                <main>
                    <form>
                        <input type="email" placeholder="Email"/>
                        <input type="password" placeholder="Password"/>
                        <button type="submit">Sign in</button>
                    </form>
                </main>
            </body></html>"#,
            "https://example.com", 1280.0, 720.0
        ).unwrap();

        let config = SerialiserConfig {
            url: "https://example.com".to_string(),
            ..Default::default()
        };

        let cct = HeadlessSerializer::serialise(&page, &config);

        assert!(cct.starts_with("##PAGE"), "CCT must start with ##PAGE header");
        assert!(cct.contains("n_"), "CCT must contain node lines");

        println!("CCT output:\n{}", &cct[..cct.len().min(500)]);
    }

    #[test]
    fn test_serialise_performance_1000_nodes() {
        let html = make_page(200);
        let page = process_html(&html, "https://bench.test", 1280.0, 720.0).unwrap();

        let config = SerialiserConfig {
            url: "https://bench.test".to_string(),
            ..Default::default()
        };

        let start = Instant::now();
        let cct = HeadlessSerializer::serialise(&page, &config);
        let elapsed = start.elapsed();

        println!("Serialised ~1000 nodes in {:?}", elapsed);
        println!("CCT size: {} bytes", cct.len());
        println!("First 200 chars:\n{}", &cct[..cct.len().min(200)]);

        assert!(cct.starts_with("##PAGE"));
    }

    #[test]
    fn test_selective_mode() {
        let page = process_html(
            r#"<html><body>
                <main>
                    <div><p>Lots of text content</p></div>
                    <form>
                        <input id="email" type="email" placeholder="Email"/>
                        <button id="submit">Login</button>
                    </form>
                </main>
            </body></html>"#,
            "https://example.com", 1280.0, 720.0
        ).unwrap();

        let config = SerialiserConfig {
            url: "https://example.com".to_string(),
            mode: SerialiserMode::Selective,
            task_hint: Some("find the login button".to_string()),
            ..Default::default()
        };

        let cct = HeadlessSerializer::serialise(&page, &config);
        assert!(cct.contains("mode=selective"));
        println!("Selective CCT:\n{}", cct);
    }

    #[test]
    fn test_page_header_format() {
        let page = process_html(
            "<html><body><p>Hello</p></body></html>",
            "https://test.com", 1280.0, 720.0
        ).unwrap();

        let config = SerialiserConfig {
            url: "https://test.com".to_string(),
            scroll_x: 0.0, scroll_y: 0.0,
            total_height: 720.0,
            ..Default::default()
        };

        let cct = HeadlessSerializer::serialise(&page, &config);
        let first_line = cct.lines().next().unwrap_or("");
        assert!(first_line.starts_with("##PAGE url=https://test.com"));
        assert!(first_line.contains("viewport=1280x720"));
        assert!(first_line.contains("mode=full"));
        println!("Page header: {}", first_line);
    }
    
    #[test]
    fn test_serialise_performance_detailed() {
        use std::time::Instant;
        let html = make_page(200);
        let page = process_html(&html, "https://perf.test", 1280.0, 720.0).unwrap();
        let config = SerialiserConfig {
            url: "https://perf.test".to_string(),
            ..Default::default()
        };

        // Warm up (first call initialises buffer pool)
        let _ = HeadlessSerializer::serialise(&page, &config);

        // Measure subsequent calls (hot path)
        let iterations = 10;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = HeadlessSerializer::serialise(&page, &config);
        }
        let total = start.elapsed();
        let avg_ms = total.as_millis() as f64 / iterations as f64;

        println!("Average serialisation time: {:.2}ms", avg_ms);
        println!("Target: <5ms (goal), <10ms (minimum)");
    }
}
