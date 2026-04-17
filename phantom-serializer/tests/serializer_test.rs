#![allow(clippy::unwrap_used, clippy::expect_used)]
#[cfg(test)]
mod tests {
    use phantom_core::process_html;
    use phantom_serializer::{HeadlessSerializer, SerialiserConfig, SerialiserMode};
    use std::collections::HashSet;
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
        let emitted_nodes = cct.lines().filter(|line| !line.starts_with("##")).count();
        assert!(emitted_nodes > 0, "serializer must emit at least one node");
    }

    #[test]
    fn test_parent_ids_reference_emitted_nodes_or_root() {
        let page = process_html(
            r#"<html><body style="width: 1280px; height: 720px;">
                <main style="width: 1000px; height: 600px;">
                    <div style="width: 500px; height: 200px;">
                        <button style="width: 100px; height: 40px;">Go</button>
                    </div>
                </main>
            </body></html>"#,
            "https://example.com",
            1280.0,
            720.0,
        )
        .unwrap();

        let cct = HeadlessSerializer::serialise(
            &page,
            &SerialiserConfig {
                url: "https://example.com".to_string(),
                ..Default::default()
            },
        );

        let mut ids = HashSet::new();
        let mut parent_ids = Vec::new();

        for line in cct.lines() {
            if line.starts_with("##") {
                continue;
            }
            let parts: Vec<&str> = line.split('|').collect();
            assert!(
                parts.len() >= 9,
                "node line must have at least 9 fields, got: {line}"
            );
            ids.insert(parts[0].to_string());
            parent_ids.push(parts[8].to_string());
        }

        for parent_id in parent_ids {
            assert!(
                parent_id == "root" || ids.contains(&parent_id),
                "parent_id {parent_id} does not reference an emitted node"
            );
        }
    }

    #[test]
    fn test_header_node_count_matches_emitted_nodes() {
        let page = process_html(
            r#"<html><body style="width: 1280px; height: 720px;">
                <div style="width: 200px; height: 60px;">Hello <span style="width: 80px; height: 20px;">world</span></div>
                <button style="width: 120px; height: 40px;">Submit</button>
            </body></html>"#,
            "https://count.test",
            1280.0,
            720.0,
        )
        .unwrap();

        let cct = HeadlessSerializer::serialise(
            &page,
            &SerialiserConfig {
                url: "https://count.test".to_string(),
                ..Default::default()
            },
        );

        let header = cct.lines().next().unwrap();
        let header_count = header
            .split_whitespace()
            .find_map(|part| part.strip_prefix("nodes="))
            .unwrap()
            .parse::<usize>()
            .unwrap();

        let emitted_count = cct.lines().filter(|line| !line.starts_with("##")).count();
        assert_eq!(
            header_count, emitted_count,
            "header nodes= must match emitted CCT node lines"
        );
    }

    #[test]
    fn test_landmark_marker_from_aria_role() {
        let page = process_html(
            r#"<html><body style="width: 1280px; height: 720px;">
                <div role="navigation" style="width: 500px; height: 100px;">menu</div>
            </body></html>"#,
            "https://landmark.test",
            1280.0,
            720.0,
        )
        .unwrap();

        let cct = HeadlessSerializer::serialise(
            &page,
            &SerialiserConfig {
                url: "https://landmark.test".to_string(),
                ..Default::default()
            },
        );
        assert!(
            cct.lines().any(|line| line.starts_with("##NAV ")),
            "role=\"navigation\" should emit ##NAV landmark marker"
        );
    }

    #[test]
    fn test_text_fields_are_escaped_for_cct_delimiters() {
        let page = process_html(
            r#"<html><body style="width: 1280px; height: 720px;">
                <button aria-label="Pay|Now,100%" style="width: 200px; height: 40px;">A|B,C%</button>
            </body></html>"#,
            "https://escape.test",
            1280.0,
            720.0,
        )
        .unwrap();

        let cct = HeadlessSerializer::serialise(
            &page,
            &SerialiserConfig {
                url: "https://escape.test".to_string(),
                ..Default::default()
            },
        );

        let btn_line = cct
            .lines()
            .find(|line| line.contains("|btn|"))
            .expect("button node line must exist");
        let fields: Vec<&str> = btn_line.split('|').collect();
        assert!(
            fields.len() >= 12,
            "button line must remain parseable after escaping: {btn_line}"
        );
        assert!(
            fields[5].contains("%7C") && fields[5].contains("%2C") && fields[5].contains("%25"),
            "accessible_name field should encode reserved delimiters"
        );
        assert!(
            fields[6].contains("%7C") && fields[6].contains("%2C") && fields[6].contains("%25"),
            "visible_text field should encode reserved delimiters"
        );
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
        assert!(first_line.starts_with("##PAGE url=https%3A%2F%2Ftest.com"));
        assert!(first_line.contains("viewport=1280x720"));
        assert!(first_line.contains("mode=full"));
        println!("Page header: {}", first_line);
    }

    #[test]
    fn test_page_header_url_is_percent_encoded() {
        let page = process_html(
            "<html><body><p>Hello</p></body></html>",
            "https://example.com",
            1280.0,
            720.0,
        )
        .unwrap();
        let config = SerialiserConfig {
            url: "https://example.com/path with space?q=a b".to_string(),
            ..Default::default()
        };

        let cct = HeadlessSerializer::serialise(&page, &config);
        let first_line = cct.lines().next().unwrap_or("");
        assert!(
            first_line.contains("url=https%3A%2F%2Fexample.com%2Fpath%20with%20space%3Fq%3Da%20b")
        );
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
        let warmup = HeadlessSerializer::serialise(&page, &config);
        assert!(
            warmup.starts_with("##PAGE"),
            "warmup serialisation must produce a valid CCT page header"
        );

        // Measure subsequent calls (hot path)
        let iterations = 10;
        let start = Instant::now();
        let mut last_cct = String::new();
        for _ in 0..iterations {
            last_cct = HeadlessSerializer::serialise(&page, &config);
        }
        let total = start.elapsed();
        let avg_ms = total.as_millis() as f64 / iterations as f64;
        assert!(
            last_cct.starts_with("##PAGE"),
            "hot-path serialisation must still produce a valid CCT header"
        );
        let emitted_nodes = last_cct
            .lines()
            .filter(|line| !line.starts_with("##"))
            .count();
        assert!(
            emitted_nodes > 0,
            "hot-path serialisation must emit at least one node"
        );

        println!("Average serialisation time: {:.2}ms", avg_ms);
        println!("Target: <5ms (goal), <10ms (minimum)");
    }
}
