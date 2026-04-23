mod client;
mod commands;
mod errors;

use std::process;

use clap::{Parser, Subcommand};
use colored::Colorize;
use serde_json::{json, Value};

use client::ServerConfig;
use errors::CliError;

#[derive(Parser)]
#[command(
    name = "ph",
    version,
    about = "Phantom Engine CLI — control and manage your headless browser engine"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// MCP server address, overrides PHANTOM_BIND_ADDR
    #[arg(long, global = true)]
    server: Option<String>,

    /// API key, overrides PHANTOM_API_KEY
    #[arg(long, global = true)]
    key: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Check connectivity to the MCP server
    Ping,

    /// Show server health, session counts, and circuit breaker status
    Status,

    /// Navigate to a URL (accepts "example.com" or "<https://example.com>")
    Navigate {
        /// Target URL
        url: String,
    },

    /// Execute JavaScript in the active page context
    Eval {
        /// JavaScript expression or statement to execute
        script: String,
    },

    /// Click an element by CSS selector
    Click {
        /// CSS selector of the target element
        selector: String,
    },

    /// Type text into an input element
    Type {
        /// CSS selector of the target input
        selector: String,
        /// Text to type
        text: String,
        /// Per-character delay in milliseconds
        #[arg(long, default_value = "50")]
        delay: u64,
    },

    /// Send a keypress event (e.g. Enter, Tab, Escape)
    Press {
        /// Key name
        key: String,
    },

    /// Retrieve the current page's scene graph (DOM state)
    SceneGraph,

    /// Tab management
    #[command(subcommand)]
    Tab(TabCommands),

    /// Cookie management
    #[command(subcommand)]
    Cookies(CookieCommands),

    /// Session persistence
    #[command(subcommand)]
    Session(SessionCommands),

    /// Local environment setup and diagnostics
    #[command(subcommand)]
    Setup(SetupCommands),

    /// Stream live DOM updates from the engine (SSE)
    Watch,

    /// Open an interactive REPL shell
    Interactive,

    /// Find elements in the page by text content
    Inspect {
        /// Text to search for in the DOM
        query: String,
    },

    /// Show available anti-detection persona profiles
    #[command(subcommand)]
    Persona(PersonaCommands),

    /// Navigation history management
    #[command(subcommand)]
    History(HistoryCommands),
}

#[derive(Subcommand)]
enum TabCommands {
    /// Open a new tab
    New {
        /// Optional URL to load in the new tab
        url: Option<String>,
    },
    /// List all open tabs
    List,
    /// Switch to a specific tab
    Switch {
        /// Tab UUID
        tab_id: String,
    },
    /// Close a tab
    Close {
        /// Tab UUID
        tab_id: String,
    },
}

#[derive(Subcommand)]
enum CookieCommands {
    /// Show all cookies
    Get,
    /// Set a cookie
    Set {
        name: String,
        value: String,
        #[arg(long)]
        domain: Option<String>,
    },
    /// Clear all cookies
    Clear,
}

#[derive(Subcommand)]
enum SessionCommands {
    /// Create a compressed snapshot of the current session
    Snapshot,
    /// Clone the current session (copy-on-write)
    Clone,
}

#[derive(Subcommand)]
enum SetupCommands {
    /// Bootstrap the local Phantom environment (~/.phantom, .env, keys)
    Init,
    /// Verify that the environment is correctly configured
    Doctor,
}

#[derive(Subcommand)]
enum PersonaCommands {
    /// List available persona profiles
    List,
}

#[derive(Subcommand)]
enum HistoryCommands {
    /// List navigation history
    List,
    /// Clear navigation history
    Clear,
}

/// Prepend https:// when the user types a bare domain like "example.com".
fn normalize_url(raw: &str) -> String {
    if raw.starts_with("http://") || raw.starts_with("https://") {
        raw.to_string()
    } else {
        format!("https://{}", raw)
    }
}

/// Pretty-print a JSON value with 2-space indent.
fn print_json(val: &Value) {
    match serde_json::to_string_pretty(val) {
        Ok(s) => println!("{}", s),
        Err(_) => println!("{}", val),
    }
}

#[tokio::main]
async fn main() {
    // Load .env if present — errors are non-fatal (the file might not exist yet)
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();
    let config = ServerConfig::resolve(cli.server.as_deref(), cli.key.as_deref());

    let result = run(cli.command, &config).await;

    if let Err(e) = result {
        eprintln!("{} {}", "error:".red().bold(), e);
        process::exit(1);
    }
}

async fn run(command: Commands, config: &ServerConfig) -> Result<(), CliError> {
    match command {
        Commands::Ping => {
            let result = client::rpc_call(config, "ping", json!({})).await?;
            println!(
                "{} {}",
                "pong".green().bold(),
                "— server is reachable".dimmed()
            );
            print_json(&result);
        }

        Commands::Status => {
            let body = client::http_get(config, "/health").await?;
            let val: Value = serde_json::from_str(&body)
                .map_err(|e| CliError::Rpc(format!("failed to parse health response: {}", e)))?;
            print_json(&val);
        }

        Commands::Navigate { url } => {
            let target = normalize_url(&url);
            println!("{} {}", "navigating".dimmed(), target.bold());
            let result =
                client::rpc_call(config, "browser_navigate", json!({ "url": target })).await?;
            print_json(&result);
        }

        Commands::Eval { script } => {
            let result =
                client::rpc_call(config, "browser_evaluate", json!({ "script": script })).await?;
            print_json(&result);
        }

        Commands::Click { selector } => {
            let result =
                client::rpc_call(config, "browser_click", json!({ "selector": selector })).await?;
            print_json(&result);
        }

        Commands::Type {
            selector,
            text,
            delay,
        } => {
            let result = client::rpc_call(
                config,
                "browser_type",
                json!({
                    "selector": selector,
                    "text": text,
                    "delay_ms": delay,
                }),
            )
            .await?;
            print_json(&result);
        }

        Commands::Press { key } => {
            let result =
                client::rpc_call(config, "browser_press_key", json!({ "key": key })).await?;
            print_json(&result);
        }

        Commands::SceneGraph => {
            let result = client::rpc_call(config, "browser_get_scene_graph", json!({})).await?;
            print_json(&result);
        }

        Commands::Tab(sub) => match sub {
            TabCommands::New { url } => {
                let target = url.map(|u| normalize_url(&u));
                let result =
                    client::rpc_call(config, "browser_new_tab", json!({ "url": target })).await?;
                print_json(&result);
            }
            TabCommands::List => {
                let result = client::rpc_call(config, "browser_list_tabs", json!({})).await?;
                print_json(&result);
            }
            TabCommands::Switch { tab_id } => {
                let result =
                    client::rpc_call(config, "browser_switch_tab", json!({ "tab_id": tab_id }))
                        .await?;
                print_json(&result);
            }
            TabCommands::Close { tab_id } => {
                let result =
                    client::rpc_call(config, "browser_close_tab", json!({ "tab_id": tab_id }))
                        .await?;
                print_json(&result);
            }
        },

        Commands::Cookies(sub) => match sub {
            CookieCommands::Get => {
                let result = client::rpc_call(config, "browser_get_cookies", json!({})).await?;
                print_json(&result);
            }
            CookieCommands::Set {
                name,
                value,
                domain,
            } => {
                let mut params = json!({ "name": name, "value": value });
                if let Some(d) = domain {
                    params["domain"] = Value::String(d);
                }
                let result = client::rpc_call(config, "browser_set_cookie", params).await?;
                print_json(&result);
            }
            CookieCommands::Clear => {
                let result = client::rpc_call(config, "browser_clear_cookies", json!({})).await?;
                print_json(&result);
            }
        },

        Commands::Session(sub) => match sub {
            SessionCommands::Snapshot => {
                let result =
                    client::rpc_call(config, "browser_session_snapshot", json!({})).await?;
                print_json(&result);
            }
            SessionCommands::Clone => {
                let result = client::rpc_call(config, "browser_session_clone", json!({})).await?;
                print_json(&result);
            }
        },

        Commands::Setup(sub) => {
            let cwd = std::env::current_dir()?;
            match sub {
                SetupCommands::Init => commands::setup::run_init(&cwd)?,
                SetupCommands::Doctor => commands::setup::run_doctor(&cwd)?,
            }
        }

        Commands::Watch => {
            commands::live::run_watch(config).await?;
        }

        Commands::Interactive => {
            commands::live::run_interactive(config).await?;
        }

        Commands::Inspect { query } => {
            commands::inspect::run_inspect(config, &query).await?;
        }

        Commands::Persona(sub) => match sub {
            PersonaCommands::List => {
                commands::persona::run_list().await?;
            }
        },

        Commands::History(sub) => match sub {
            HistoryCommands::List => {
                commands::history::run_list().await?;
            }
            HistoryCommands::Clear => {
                commands::history::run_clear().await?;
            }
        },
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::normalize_url;

    #[test]
    fn bare_domain_gets_https_prefix() {
        assert_eq!(normalize_url("example.com"), "https://example.com");
        assert_eq!(
            normalize_url("google.com/search"),
            "https://google.com/search"
        );
    }

    #[test]
    fn explicit_scheme_left_untouched() {
        assert_eq!(normalize_url("https://example.com"), "https://example.com");
        assert_eq!(
            normalize_url("http://localhost:3000"),
            "http://localhost:3000"
        );
    }
}
