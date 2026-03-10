use crate::config::Config;
use crate::db::Database;
use crate::error::RecallErrors;
use colored::Colorize;

/// Entry point for `recall undo`.
///
/// Removes the most recently logged command from the database and prints
/// what was removed so the user can confirm it was the right entry.
///
/// The primary use case is accidentally logging a sensitive value — for
/// example a password typed as a command. Running `recall undo` immediately
/// removes it before it can be exported or searched.
///
/// # Errors
/// - [`RecallErrors::NoHomeDir`] if the home directory cannot be determined.
/// - [`RecallErrors::Database`] if the delete fails.
pub fn run() -> Result<(), RecallErrors> {
    let config = Config::load()?;
    let db = Database::open(&config.db_path)?;

    match db.undo()? {
        Some(cmd) => {
            // Show the removed command and how long ago it was logged so the
            // user can confirm it was the right entry.
            println!(
                "{}",
                format!(
                    "✓ Removed: {} (logged {})",
                    cmd.command,
                    cmd.relative_time()
                )
                .green()
            );
        }
        None => {
            // Database is empty — nothing to remove.
            println!("{}", "Nothing to undo — history is empty.".yellow());
        }
    }

    Ok(())
}
