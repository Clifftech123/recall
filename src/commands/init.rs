use crate::config::Config;
use crate::db::Database;
use crate::error::RecallErrors;
use colored::Colorize;

/// Entry point for `recall init`.
///
/// Creates the `~/.recall/` directory and initialises the SQLite database.
/// Safe to run multiple times — if the database already exists it prints
/// a message and exits without touching anything.
///
/// # Errors
/// - [`RecallErrors::NoHomeDir`] if the home directory cannot be determined.
/// - [`RecallErrors::Database`] if the database cannot be created or migrated.
pub fn run() -> Result<(), RecallErrors> {
    let config = Config::load()?;

    // If already initialised just let the user know and exit cleanly.
    // No need to error — running init twice is not a mistake.
    if config.is_initialized() {
        println!(
            "{}",
            format!(
                "✓ Recall is already initialised at {}",
                config.data_dir.display()
            )
            .yellow()
        );
        return Ok(());
    }

    // Create ~/.recall/ directory before opening the database.
    // Database::open will fail if the directory does not exist.
    config.ensure_dir()?;

    // Opening the database runs migrate() internally which creates all
    // tables, indexes, triggers, and the FTS5 virtual table.
    // The ? propagates any database error up to the caller.
    Database::open(&config.db_path)?;

    println!(
        "{}",
        format!("✓ Recall initialised at {}", config.data_dir.display()).green()
    );
    println!("  Run 'recall hook' to start automatic logging.");

    Ok(())
}
