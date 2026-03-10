use crate::error::RecallErrors;
use crate::shell;
use std::fs;

/// Entry point for `recall hook`.
///
/// Detects the current shell, finds its config file, checks the hook is not
/// already installed, writes a backup, then appends the hook script block.
///
/// Fish is handled slightly differently — its config goes into a dedicated
/// drop-in file (`~/.config/fish/conf.d/recall.fish`) which may not exist
/// yet. If the parent directory does not exist it is created automatically.
///
/// # Errors
/// - [`RecallErrors::UnknownShell`] if the shell cannot be detected.
/// - [`RecallErrors::NoHomeDir`] if the home directory cannot be determined.
/// - [`RecallErrors::HookAlreadyInstalled`] if the hook is already present.
/// - [`RecallErrors::ShellConfigRead`] if the config file cannot be read.
/// - [`RecallErrors::ShellConfigWrite`] if the backup or updated file cannot be written.
pub fn run() -> Result<(), RecallErrors> {
    // hook does not need the database — only shell detection and file I/O.
    let shell = shell::detect()?;
    let config_path = shell::config_path(&shell)?;

    // Create parent directories if they do not exist.
    // This matters for Fish — conf.d/ may not exist on a fresh install.
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|e| RecallErrors::ShellConfigRead {
            path: parent.display().to_string(),
            reason: e.to_string(),
        })?;
    }

    // Read the existing config file into memory.
    // If the file does not exist yet (Fish conf.d case) start with an empty
    // string — install_hook will produce a file containing only the hook block.
    let existing = if config_path.exists() {
        fs::read_to_string(&config_path).map_err(|e| RecallErrors::ShellConfigRead {
            path: config_path.display().to_string(),
            reason: e.to_string(),
        })?
    } else {
        String::new()
    };

    // Guard: do not install twice.
    // is_hook_installed searches for the HOOK_START marker in the file content.
    if shell::is_hook_installed(&existing) {
        return Err(RecallErrors::HookAlreadyInstalled(
            config_path.display().to_string(),
        ));
    }

    // Build the backup path by appending ".recall.bak" to the full filename.
    // with_extension() mangles dotfiles like .bashrc so we push the suffix
    // onto the OsString filename directly instead.
    let backup_path = {
        let mut name = config_path.file_name().unwrap_or_default().to_os_string();
        name.push(".recall.bak");
        config_path.with_file_name(name)
    };

    // Only write a backup if the config file already existed.
    // No point backing up an empty file we are about to create.
    // Track this with a flag so the success message knows whether to show the backup path.
    let had_existing_file = config_path.exists();
    if had_existing_file {
        fs::write(&backup_path, &existing).map_err(|e| RecallErrors::ShellConfigWrite {
            path: backup_path.display().to_string(),
            reason: e.to_string(),
            backup: backup_path.display().to_string(),
        })?;
    }

    // Get the hook script for the detected shell.
    // The script already contains the HOOK_START and HOOK_END markers.
    // install_hook appends it to the existing content with a blank separator.
    let script = shell::hook_script(&shell);
    let updated = shell::install_hook(&existing, script);

    // Write the updated config file back to disk.
    fs::write(&config_path, updated).map_err(|e| RecallErrors::ShellConfigWrite {
        path: config_path.display().to_string(),
        reason: e.to_string(),
        backup: backup_path.display().to_string(),
    })?;

    println!(
        "✓ Hook installed for {} in {}",
        shell,
        config_path.display()
    );

    // Only show the backup line if we actually created one.
    if had_existing_file {
        println!("  Backup saved to {}", backup_path.display());
    }

    println!();

    // Tell the user how to activate the hook without restarting their shell.
    match shell {
        crate::shell::Shell::Bash => {
            println!("  Restart your shell or run:  source ~/.bashrc")
        }
        crate::shell::Shell::Zsh => {
            println!("  Restart your shell or run:  source ~/.zshrc")
        }
        crate::shell::Shell::Fish => {
            println!("  Open a new Fish session to apply changes.")
        }
        crate::shell::Shell::PowerShell => {
            println!("  Restart PowerShell to apply changes.")
        }
    }

    Ok(())
}
