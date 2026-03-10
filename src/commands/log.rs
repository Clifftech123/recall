use crate::config::Config;
use crate::db::Database;
use crate::error::RecallErrors;
use crate::models::Command;
use chrono::Utc;

/// Entry point for `recall log`.
///
/// This is the hot path — it runs silently after every single command the
/// user types via the shell hook. It must be fast and produce no output.
///
/// Any error is propagated up to `main.rs` where the log command swallows
/// it silently. Errors must never leak into the user's terminal.
///
/// # Errors
/// - [`RecallErrors::NoHomeDir`] if the home directory cannot be determined.
/// - [`RecallErrors::Database`] if the insert fails.
pub fn run(
    command: String,
    session: Option<String>,
    cwd: Option<String>,
    exit_code: i32,
    shell: Option<String>,
    metadata: Option<String>,
) -> Result<(), RecallErrors> {
    let config = Config::load()?;

    // Resolve the hostname once at log time so each row knows which machine
    // it came from. Falls back to "unknown" if the OS call fails — never fatal.
    let hostname = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string());

    // id is set to 0 here — the database assigns the real AUTOINCREMENT id
    // on insert. The value here is never written to the database.
    let cmd = Command {
        id: 0,
        command,
        timestamp: Utc::now(),
        session_id: session,
        cwd,
        exit_code,
        shell,
        hostname: Some(hostname),
        metadata,
    };

    // Open the database and insert. If the database does not exist yet
    // (recall init has not been run), Database::open returns an error which
    // propagates via ? and is silently swallowed by the caller in main.rs.
    let db = Database::open(&config.db_path)?;
    db.insert(&cmd)?;

    Ok(())
}
