use crate::config::Config;
use crate::db::Database;
use crate::error::RecallErrors;
use crate::format;
use colored::Colorize;

/// Entry point for `recall stats`.
///
/// Loads aggregated usage data from the database and prints a formatted
/// statistics dashboard to stdout.
///
/// Guards against an empty database — printing a dashboard full of zeros
/// looks broken and is not useful to the user.
///
/// # Errors
/// - [`RecallErrors::NoHomeDir`] if the home directory cannot be determined.
/// - [`RecallErrors::Database`] if any aggregation query fails.
pub fn run(top: u64) -> Result<(), RecallErrors> {
    let config = Config::load()?;
    let db = Database::open(&config.db_path)?;

    let stats = db.stats(top)?;

    // Guard: if there are no commands logged yet the dashboard is meaningless.
    // Point the user to the hook command so they know what to do next.
    if stats.total_commands == 0 {
        println!(
            "{}",
            "No commands logged yet. Run 'recall hook' to start automatic logging.".yellow()
        );
        return Ok(());
    }

    format::as_stats(&stats);

    Ok(())
}
