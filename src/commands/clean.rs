use crate::config::Config;
use crate::db::Database;
use crate::error::RecallErrors;
use colored::Colorize;

/// Entry point for `recall clean`.
///
/// Validates the arguments, opens the database, delegates to
/// [`Database::clean`], and prints a clear result to the user.
///
/// # Argument rules
/// - `--all` and `--from`/`--to` cannot be used together.
/// - If neither `--all` nor any date bound is given, nothing is deleted.
/// - `--dry-run` prints how many rows *would* be deleted without touching the database.
///
/// # Errors
/// - [`RecallErrors::ConflictingCleanFlags`] if `--all` is combined with a date bound.
/// - [`RecallErrors::Database`] if the database cannot be opened or the query fails.
pub fn run(
    from: Option<&str>,
    to: Option<&str>,
    all: bool,
    dry_run: bool,
) -> Result<(), RecallErrors> {
    // Guard: --all cannot be combined with --from or --to.
    // The two approaches are mutually exclusive — either wipe everything
    // or wipe a specific date range, never both at once.
    if all && (from.is_some() || to.is_some()) {
        return Err(RecallErrors::ConflictingCleanFlags);
    }

    // Guard: if nothing was specified there is nothing to do.
    if !all && from.is_none() && to.is_none() {
        println!(
            "{}",
            "Nothing to clean. Use --all or provide --from / --to.".yellow()
        );
        return Ok(());
    }

    let config = Config::load()?;
    let db = Database::open(&config.db_path)?;

    // db.clean returns the number of rows deleted (or that would be deleted).
    let count = db.clean(from, to, all, dry_run)?;

    if dry_run {
        // Dry run — tell the user what would happen without doing it.
        println!(
            "{}",
            format!(
                "Would delete {} {}. Run without --dry-run to confirm.",
                count,
                if count == 1 { "command" } else { "commands" }
            )
            .yellow()
        );
    } else if count == 0 {
        // Nothing matched the given range.
        println!("{}", "No commands matched. Nothing was deleted.".dimmed());
    } else {
        // Deletion happened — confirm with a count so the user knows what was removed.
        println!(
            "{}",
            format!(
                "✓ Deleted {} {}.",
                count,
                if count == 1 { "command" } else { "commands" }
            )
            .green()
        );
    }

    Ok(())
}
