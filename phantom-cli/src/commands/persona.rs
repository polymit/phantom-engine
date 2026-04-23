use colored::Colorize;

use crate::errors::CliError;

/// `phantom persona list` — show available anti-detection profiles.
pub async fn run_list() -> Result<(), CliError> {
    // The MCP server doesn't expose a persona listing endpoint yet,
    // so we describe what the engine generates internally.
    println!("{}\n", "Available Persona Profiles".bold());
    println!("  The engine auto-generates D-60 compliant personas at session creation.");
    println!("  Each persona includes:\n");
    println!(
        "    • {} — randomized from common desktop browsers",
        "User-Agent".cyan()
    );
    println!(
        "    • {} — bucketed to real-world distributions",
        "Hardware Concurrency".cyan()
    );
    println!("    • {} — bucketed (4/8/16 GB)", "Device Memory".cyan());
    println!(
        "    • {} — seeded with crypto-quality entropy",
        "Canvas Noise".cyan()
    );
    println!(
        "    • {} — realistic viewport + screen dimensions",
        "Screen Geometry".cyan()
    );

    println!(
        "\n  {}",
        "Persona selection via CLI will be available in v0.2.".dimmed()
    );

    Ok(())
}
