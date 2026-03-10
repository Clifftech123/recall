use crate::config::Config;
use crate::db::Database;
use crate::error::RecallErrors;
use crate::format;
use colored::Colorize;

/// Entry point for `recall search`.
///
/// Runs a full-text search against the FTS5 index and prints the results
/// in the requested format. If no results are found a clear message is
/// shown regardless of the output format.
///
/// # Errors
/// - [`RecallErrors::NoHomeDir`] if the home directory cannot be determined.
/// - [`RecallErrors::Database`] if the search query fails.
/// - [`RecallErrors::Json`] if JSON serialisation fails.
/// - [`RecallErrors::Csv`] if CSV writing fails.
pub fn run(query: &str, limit: u64, json: bool, csv: bool) -> Result<(), RecallErrors> {
    let config = Config::load()?;
    let db = Database::open(&config.db_path)?;

    let commands = db.search(query, limit)?;

    // Give clear feedback when nothing matched rather than printing an empty
    // table, empty JSON array, or blank CSV — all of which look like a bug.
    if commands.is_empty() {
        println!(
            "{}",
            format!("No results found for \"{}\".", query).yellow()
        );
        return Ok(());
    }

    if json {
        format::as_json(&commands)?;
    } else if csv {
        format::as_csv(&commands)?;
    } else {
        // Default: coloured table with result count header.
        println!(
            "{}",
            format!(
                "  Found {} {} for \"{}\"",
                commands.len(),
                if commands.len() == 1 {
                    "result"
                } else {
                    "results"
                },
                query
            )
            .dimmed()
        );
        format::as_table(&commands);
    }

    Ok(())
}
