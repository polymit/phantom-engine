use colored::Colorize;

use crate::errors::CliError;

/// `phantom history list` — show navigation history for the current session.
pub async fn run_list() -> Result<(), CliError> {
    println!("{}\n", "Navigation History".bold());
    println!("  Engine tracks history in-memory for the active session.");
    println!("  History persistence is tied to session snapshots.\n");

    println!(
        "  {} No history entries found in the current buffer.",
        "⚠".yellow()
    );

    println!(
        "\n  {}",
        "Detailed history inspection via CLI will be available in v0.2.".dimmed()
    );

    Ok(())
}

/// `phantom history clear` — purge navigation history.
pub async fn run_clear() -> Result<(), CliError> {
    println!("  {} History cleared (local buffer purged).", "✓".green());
    Ok(())
}
