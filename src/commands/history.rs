use crate::config::Config;
use crate::db::Database;
use crate::error::RecallErrors;
use crate::format;
use colored::Colorize;

/// Entry point for `recall history`.
///
/// Fetches recent commands from the database and prints them in the
/// requested format. Optional filters narrow the results by session,
/// working directory, or failure status.
///
/// # Errors
/// - [`RecallErrors::NoHomeDir`] if the home directory cannot be determined.
/// - [`RecallErrors::Database`] if the query fails.
/// - [`RecallErrors::Json`] if JSON serialisation fails.
/// - [`RecallErrors::Csv`] if CSV writing fails.
pub fn run(
    limit: u64,
    json: bool,
    csv: bool,
    session: Option<String>,
    cwd: Option<String>,
    errors: bool,
) -> Result<(), RecallErrors> {
    let config = Config::load()?;
    let db = Database::open(&config.db_path)?;

    // Pass filters as &str references — db.history takes Option<&str> so we
    // use as_deref() to convert Option<String> without consuming the values.
    let commands = db.history(limit, session.as_deref(), cwd.as_deref(), errors)?;

    // Give clear feedback when filters produce no results rather than printing
    // an empty table, empty JSON array, or blank CSV.
    if commands.is_empty() {
        let reason = build_empty_reason(session.as_deref(), cwd.as_deref(), errors);
        println!("{}", reason.yellow());
        return Ok(());
    }

    if json {
        format::as_json(&commands)?;
    } else if csv {
        format::as_csv(&commands)?;
    } else {
        format::as_table(&commands);
    }

    Ok(())
}

/// Builds a human-readable explanation for why the history query returned
/// no results based on which filters were active.
///
/// This avoids a generic "No commands found." message when the user has
/// applied filters — instead they get a message that reflects exactly
/// what was searched so they know what to adjust.
fn build_empty_reason(session: Option<&str>, cwd: Option<&str>, errors: bool) -> String {
    // Collect active filter descriptions to include in the message.
    let mut filters: Vec<String> = Vec::new();

    if let Some(s) = session {
        filters.push(format!("session \"{}\"", s));
    }
    if let Some(c) = cwd {
        filters.push(format!("directory \"{}\"", c));
    }
    if errors {
        filters.push("failed commands".to_string());
    }

    if filters.is_empty() {
        // No filters were active — the database is simply empty.
        "No commands logged yet. Run 'recall hook' to start automatic logging.".to_string()
    } else {
        // Filters were active — tell the user exactly what matched nothing.
        format!("No commands found for {}.", filters.join(", "))
    }
}
