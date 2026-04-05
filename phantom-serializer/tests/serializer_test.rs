#[cfg(test)]
mod tests {
    use phantom_core::process_html;
    use phantom_serializer::{HeadlessSerializer, SerialiserConfig, SerialiserMode};
    use std::time::Instant;

    fn make_page(n: usize) -> String {
        let mut html = String::from("<html><body style='width: 1280px; height: 720px;'>");
        for i in 0..n {
            html.push_str(&format!(
                "<div id='node{}' style='width: 100px; height: 100px;'><button style='width: 50px; height: 20px;'>Btn {}</button>\
                 <input type='text' placeholder='Field {}' style='width: 80px; height: 20px;'/>\
                 </div>", i, i, i
            ));
        }
        html.push_str("</body></html>");
        html
    }

    #[test]
    fn test_serialise_basic_page() {
        let page = process_html(
            r#"<html><body style="width: 1280px; height: 720px;">
                <nav style="width: 1280px; height: 50px;"><a href="/" style="width: 50px; height: 20px;">Home</a></nav>
                <main style="width: 1000px; height: 600px;">
                    <form style="width: 400px; height: 300px;">
                        <input type="email" placeholder="Email" style="width: 200px; height: 30px;"/>
                        <input type="password" placeholder="Password" style="width: 200px; height: 30px;"/>
                        <button type="submit" style="width: 100px; height: 40px;">Sign in</button>
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

        assert!(
            cct.starts_with("##PAGE"),
            "CCT must start with ##PAGE header"
        );
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
            r#"<html><body style="width: 1280px; height: 720px;">
                <main style="width: 800px; height: 600px;">
                    <div style="width: 400px; height: 100px;"><p style="width: 200px; height: 20px;">Lots of text content</p></div>
                    <form style="width: 400px; height: 300px;">
                        <input id="email" type="email" placeholder="Email" style="width: 200px; height: 30px;"/>
                        <button id="submit" style="width: 100px; height: 40px;">Login</button>
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
            "<html><body style='width: 1280px; height: 720px;'><p style='width: 100px; height: 20px;'>Hello</p></body></html>",
            "https://test.com", 1280.0, 720.0
        ).unwrap();

        let config = SerialiserConfig {
            url: "https://test.com".to_string(),
            scroll_x: 0.0,
            scroll_y: 0.0,
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
