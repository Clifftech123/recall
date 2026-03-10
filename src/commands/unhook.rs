use crate::error::RecallErrors;
use crate::shell;
use colored::Colorize;
use std::fs;

/// Entry point for `recall unhook`.
///
/// Detects the current shell, finds its config file, strips the recall hook
/// block from it, and saves a backup before making any changes.
///
/// Fish is handled separately — because recall writes a dedicated file for
/// Fish (`conf.d/recall.fish`), unhook simply deletes that file entirely
/// rather than editing it.
///
/// # Errors
/// - [`RecallErrors::UnknownShell`] if the shell cannot be detected.
/// - [`RecallErrors::NoHomeDir`] if the home directory cannot be determined.
/// - [`RecallErrors::ShellConfigRead`] if the config file cannot be read.
/// - [`RecallErrors::ShellConfigWrite`] if the backup or cleaned file cannot be written.
pub fn run() -> Result<(), RecallErrors> {
    // unhook does not touch the database at all — no Config or Database needed.
    // We only need the shell detection and file paths.
    let shell = shell::detect()?;
    let config_path = shell::config_path(&shell)?;

    // Nothing to do if the config file does not exist at all.
    if !config_path.exists() {
        println!(
            "{}",
            format!(
                "No shell config found at {}. Nothing to remove.",
                config_path.display()
            )
            .yellow()
        );
        return Ok(());
    }

    // Read the current config file into memory before touching anything.
    let existing = fs::read_to_string(&config_path).map_err(|e| RecallErrors::ShellConfigRead {
        path: config_path.display().to_string(),
        reason: e.to_string(),
    })?;

    // Bail early if the hook was never installed — nothing to strip.
    if !shell::is_hook_installed(&existing) {
        println!(
            "{}",
            format!(
                "No recall hook found in {}. Nothing to remove.",
                config_path.display()
            )
            .yellow()
        );
        return Ok(());
    }

    // Build the backup path by appending ".recall.bak" to the full filename.
    // Using with_extension() would mangle files like .bashrc (no real extension)
    // so we push the suffix onto the OsString filename directly instead.
    let backup_path = {
        let mut name = config_path.file_name().unwrap_or_default().to_os_string();
        name.push(".recall.bak");
        config_path.with_file_name(name)
    };

    // Write the backup before modifying anything — if the write below fails
    // the original file is untouched and the backup still exists.
    fs::write(&backup_path, &existing).map_err(|e| RecallErrors::ShellConfigWrite {
        path: backup_path.display().to_string(),
        reason: e.to_string(),
        backup: backup_path.display().to_string(),
    })?;

    // ── Fish: delete the dedicated conf.d file entirely ───────────────────────
    //
    // recall hook wrote a standalone file for Fish rather than appending to
    // an existing config, so the clean removal is to delete the file outright.
    if shell == shell::Shell::Fish {
        fs::remove_file(&config_path).map_err(|e| RecallErrors::ShellConfigWrite {
            path: config_path.display().to_string(),
            reason: e.to_string(),
            backup: backup_path.display().to_string(),
        })?;

        println!(
            "{}",
            format!("✓ Hook removed — deleted {}", config_path.display()).green()
        );
        println!(
            "  {}",
            format!("Backup saved to {}", backup_path.display()).dimmed()
        );
        return Ok(());
    }

    // ── All other shells: strip the hook block and write the file back ────────
    //
    // remove_hook finds the lines between HOOK_START and HOOK_END markers and
    // removes them, leaving the rest of the config file exactly as it was.
    let cleaned = shell::remove_hook(&existing);

    fs::write(&config_path, cleaned).map_err(|e| RecallErrors::ShellConfigWrite {
        path: config_path.display().to_string(),
        reason: e.to_string(),
        backup: backup_path.display().to_string(),
    })?;

    println!(
        "{}",
        format!("✓ Hook removed from {}", config_path.display()).green()
    );
    println!(
        "  {}",
        format!("Backup saved to {}", backup_path.display()).dimmed()
    );
    println!();

    // Tell the user how to apply the change for their specific shell.
    match shell {
        shell::Shell::Bash => {
            println!(
                "  Restart your shell or run:  {}",
                "source ~/.bashrc".cyan()
            )
        }
        shell::Shell::Zsh => {
            println!("  Restart your shell or run:  {}", "source ~/.zshrc".cyan())
        }
        shell::Shell::PowerShell => {
            println!("  Restart PowerShell to apply changes.")
        }
        // Already handled above — Fish returns early before reaching this match.
        shell::Shell::Fish => unreachable!(),
    }

    Ok(())
}
