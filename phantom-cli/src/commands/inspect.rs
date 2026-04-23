use colored::Colorize;
use serde_json::json;

use crate::client::{self, ServerConfig};
use crate::errors::CliError;

/// `phantom inspect <query>` — find elements by text content or accessibility label.
///
/// Uses the scene graph's text nodes to locate elements matching the query,
/// returning their CSS selectors and bounding boxes.
pub async fn run_inspect(config: &ServerConfig, query: &str) -> Result<(), CliError> {
    println!(
        "{} Searching for: {}",
        "inspect".cyan().bold(),
        query.bold()
    );

    // Grab the full scene graph, then search client-side.
    // This avoids adding a new MCP method for v0.1 — we can optimize later.
    let scene = client::rpc_call(config, "browser_get_scene_graph", json!({})).await?;

    let cct = scene.get("cct").and_then(|v| v.as_str()).unwrap_or("");

    let query_lower = query.to_lowercase();
    let mut matches: Vec<String> = Vec::new();

    // CCT lines that contain the query text are likely the elements the user wants
    for line in cct.lines() {
        if line.to_lowercase().contains(&query_lower) {
            matches.push(line.trim().to_string());
        }
    }

    if matches.is_empty() {
        println!("  {} No elements matching '{}'", "⚠".yellow(), query);
    } else {
        println!("  {} {} match(es):\n", "✓".green(), matches.len());
        for (i, m) in matches.iter().enumerate() {
            println!("  {}. {}", i + 1, m);
        }
    }

    Ok(())
}
