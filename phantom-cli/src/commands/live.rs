use std::io::{self, Write};

use colored::Colorize;
use serde_json::{json, Value};

use crate::client::{self, ServerConfig};
use crate::errors::CliError;

/// `phantom watch` — subscribe to the SSE delta stream and print events live.
///
/// Connects to /sse and prints each server-sent event as it arrives.
/// Ctrl+C to stop.
pub async fn run_watch(config: &ServerConfig) -> Result<(), CliError> {
    println!(
        "{} Connecting to SSE stream at {}/sse …",
        "watch".green().bold(),
        config.base_url
    );
    println!("{}", "Press Ctrl+C to stop.\n".dimmed());

    let body = client::http_get(config, "/sse").await?;

    // SSE frames are newline-delimited "data: <payload>" lines
    for line in body.lines() {
        if let Some(payload) = line.strip_prefix("data: ") {
            println!("{} {}", "δ".cyan(), payload);
        }
    }

    Ok(())
}

/// `phantom interactive` — REPL loop that accepts commands one at a time.
///
/// Keeps a persistent connection context so tab/cookie state carries
/// between commands without re-specifying --server and --key.
pub async fn run_interactive(config: &ServerConfig) -> Result<(), CliError> {
    println!("{}", "Phantom Interactive Shell".green().bold());
    println!(
        "{}",
        "Type 'help' for available commands, 'quit' to exit.\n".dimmed()
    );

    let mut line_buf = String::new();
    loop {
        print!("{} ", "ph>".cyan().bold());
        io::stdout().flush()?;

        line_buf.clear();
        if io::stdin().read_line(&mut line_buf)? == 0 {
            break; // EOF
        }

        let input = line_buf.trim();
        if input.is_empty() {
            continue;
        }

        match input {
            "quit" | "exit" | "q" => break,
            "help" | "?" => print_repl_help(),
            _ => {
                if let Err(e) = dispatch_repl_command(config, input).await {
                    eprintln!("{} {}", "error:".red(), e);
                }
            }
        }
    }

    println!("{}", "goodbye.".dimmed());
    Ok(())
}

fn print_repl_help() {
    println!(
        r#"
  {}        — check server connectivity
  {} <url>  — navigate to a page
  {} <js>   — run JavaScript
  {}  — get the current DOM state
  {}       — show server health
  {}      — exit the shell
"#,
        "ping".bold(),
        "navigate".bold(),
        "eval".bold(),
        "scene-graph".bold(),
        "status".bold(),
        "quit".bold(),
    );
}

async fn dispatch_repl_command(config: &ServerConfig, input: &str) -> Result<(), CliError> {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    let cmd = parts[0];
    let arg = parts.get(1).copied().unwrap_or("");

    let result = match cmd {
        "ping" => client::rpc_call(config, "ping", json!({})).await?,
        "navigate" => {
            if arg.is_empty() {
                return Err(CliError::Rpc("usage: navigate <url>".to_string()));
            }
            let url = crate::normalize_url(arg);
            client::rpc_call(config, "browser_navigate", json!({ "url": url })).await?
        }
        "eval" => {
            if arg.is_empty() {
                return Err(CliError::Rpc("usage: eval <script>".to_string()));
            }
            client::rpc_call(config, "browser_evaluate", json!({ "script": arg })).await?
        }
        "scene-graph" => client::rpc_call(config, "browser_get_scene_graph", json!({})).await?,
        "status" => {
            let body = client::http_get(config, "/health").await?;
            let val: Value = serde_json::from_str(&body)
                .map_err(|e| CliError::Rpc(format!("bad health response: {}", e)))?;
            val
        }
        _ => {
            return Err(CliError::Rpc(format!(
                "unknown command: '{}' — type 'help'",
                cmd
            )));
        }
    };

    crate::print_json(&result);
    Ok(())
}
