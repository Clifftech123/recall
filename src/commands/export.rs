use crate::config::Config;
use crate::db::Database;
use crate::error::RecallErrors;
use crate::models::Command;
use colored::Colorize;
use std::fs::File;
use std::io::{self, Write};

/// Entry point for `recall export`.
///
/// Exports the full command history in the requested format to either a
/// file or stdout. The export is ordered oldest to newest so the output
/// reads chronologically.
///
/// Supported formats:
// - `json`  — pretty-printed JSON array (default)
/// - `csv`   — comma-separated values with a header row
/// - `text`  — plain text, one line per command
///
/// # Errors
/// - [`RecallErrors::NoHomeDir`] if the home directory cannot be determined.
/// - [`RecallErrors::Database`] if the query fails.
/// - [`RecallErrors::InvalidFormat`] if an unrecognised format string is given.
/// - [`RecallErrors::Io`] if the output file cannot be created or written.
/// - [`RecallErrors::Json`] if JSON serialisation fails.
/// - [`RecallErrors::Csv`] if CSV writing fails.
pub fn run(fmt: Option<&str>, output: Option<&str>) -> Result<(), RecallErrors> {
    let config = Config::load()?;
    let db = Database::open(&config.db_path)?;

    let commands = db.export()?;

    // Guard: nothing to export — tell the user rather than writing an empty file.
    if commands.is_empty() {
        println!(
            "{}",
            "No commands to export. Run 'recall hook' to start automatic logging.".yellow()
        );
        return Ok(());
    }

    // Default to JSON if no format was specified.
    let format = fmt.unwrap_or("json");

    // Validate the format before opening any file so we fail fast and cleanly.
    validate_format(format)?;

    match output {
        // ── Write to a file ───────────────────────────────────────────────────
        Some(path) => {
            // Create (or truncate) the output file.
            let file = File::create(path).map_err(RecallErrors::Io)?;
            write_export(format, &commands, file)?;
            println!(
                "{}",
                format!(
                    "✓ Exported {} {} to {} ({})",
                    commands.len(),
                    if commands.len() == 1 {
                        "command"
                    } else {
                        "commands"
                    },
                    path,
                    format
                )
                .green()
            );
        }

        // ── Write to stdout ───────────────────────────────────────────────────
        // No success message here — the output itself is the response.
        None => {
            write_export(format, &commands, io::stdout())?;
        }
    }

    Ok(())
}

/// Checks that `format` is one of the supported export formats.
///
/// Separated from `write_export` so we can validate and return a clear error
/// before any file is created or any output is written.
///
/// # Errors
/// Returns [`RecallErrors::InvalidFormat`] if the format is not recognised.
fn validate_format(format: &str) -> Result<(), RecallErrors> {
    match format {
        "json" | "csv" | "text" => Ok(()),
        other => Err(RecallErrors::InvalidFormat {
            input: other.to_string(),
        }),
    }
}

/// Writes all commands to `writer` in the requested format.
///
/// `writer` is generic over [`Write`] so the same function handles both
/// file output and stdout without duplication.
///
/// Assumes `format` has already been validated by [`validate_format`].
fn write_export<W: Write>(
    format: &str,
    commands: &[Command],
    mut writer: W,
) -> Result<(), RecallErrors> {
    match format {
        "json" => {
            // Pretty-print so the file is human-readable, not one long line.
            let json = serde_json::to_string_pretty(commands)?;
            writer.write_all(json.as_bytes())?;
            // Ensure the file ends with a newline — standard Unix convention.
            writer.write_all(b"\n")?;
        }

        "csv" => {
            let mut csv_writer = csv::Writer::from_writer(writer);

            // Header row.
            csv_writer.write_record(&[
                "id",
                "command",
                "timestamp",
                "session_id",
                "cwd",
                "exit_code",
                "shell",
                "hostname",
            ])?;

            // One data row per command. Optional fields written as empty
            // strings so the column count stays consistent throughout the file.
            for cmd in commands {
                csv_writer.write_record(&[
                    cmd.id.to_string(),
                    cmd.command.clone(),
                    cmd.timestamp.to_rfc3339(),
                    cmd.session_id.clone().unwrap_or_default(),
                    cmd.cwd.clone().unwrap_or_default(),
                    cmd.exit_code.to_string(),
                    cmd.shell.clone().unwrap_or_default(),
                    cmd.hostname.clone().unwrap_or_default(),
                ])?;
            }

            // Flush ensures the final record is written even if the
            // underlying writer is buffered.
            csv_writer.flush()?;
        }

        "text" => {
            // One line per command: [timestamp]  cwd  command
            // Simple format — easy to grep, diff, or read in any editor.
            for cmd in commands {
                let line = format!(
                    "[{}]  {}  {}\n",
                    cmd.timestamp.format("%Y-%m-%d %H:%M:%S"),
                    cmd.display_cwd(),
                    cmd.command
                );
                writer.write_all(line.as_bytes())?;
            }
        }

        // validate_format guarantees we never reach this branch.
        _ => unreachable!("format was already validated"),
    }

    Ok(())
}
