use crate::error::RecallErrors;
use std::path::PathBuf;

/// The marker line written at the start of the hook block.
/// Used by [`is_hook_installed`] and [`remove_hook`] to find the block.
const HOOK_START: &str = "# >>> recall hook start >>>";

/// The marker line written at the end of the hook block.
const HOOK_END: &str = "# <<< recall hook end <<<";

// ── Shell enum ────────────────────────────────────────────────────────────────

/// The shells Recall knows how to install a hook into.
#[derive(Debug, Clone, PartialEq)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
}

impl std::fmt::Display for Shell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Shell::Bash => write!(f, "bash"),
            Shell::Zsh => write!(f, "zsh"),
            Shell::Fish => write!(f, "fish"),
            Shell::PowerShell => write!(f, "powershell"),
        }
    }
}

// ── Detection ─────────────────────────────────────────────────────────────────

/// Detects the shell that is currently running.
///
/// Detection order:
/// 1. `$SHELL` environment variable — the most reliable signal on Unix.
///    Checked first so a Unix user who happens to have PowerShell installed
///    does not get misidentified.
/// 2. `$PSModulePath` — set by PowerShell on all platforms. Checked second
///    as a fallback for Windows where `$SHELL` is not set.
///
/// # Errors
/// Returns [`RecallErrors::UnknownShell`] if neither check produces a match.
pub fn detect() -> Result<Shell, RecallErrors> {
    // Check $SHELL first — authoritative on Unix and avoids false-positives
    // from $PSModulePath being present as a leftover variable in bash/zsh.
    if let Ok(shell) = std::env::var("SHELL") {
        if shell.contains("bash") {
            return Ok(Shell::Bash);
        }
        if shell.contains("zsh") {
            return Ok(Shell::Zsh);
        }
        if shell.contains("fish") {
            return Ok(Shell::Fish);
        }
    }

    // $PSModulePath is set by PowerShell on all platforms.
    // Only reached if $SHELL was absent or unrecognised (i.e. on Windows).
    if std::env::var("PSModulePath").is_ok() {
        return Ok(Shell::PowerShell);
    }

    Err(RecallErrors::UnknownShell)
}

// ── Config file paths ─────────────────────────────────────────────────────────

/// Returns the shell config file path that Recall will modify for the given shell.
///
/// | Shell       | Path |
/// |-------------|------|
/// | bash        | `~/.bashrc` |
/// | zsh         | `~/.zshrc` |
/// | fish        | `~/.config/fish/conf.d/recall.fish` |
/// | PowerShell  | `~/Documents/PowerShell/Microsoft.PowerShell_profile.ps1` (Windows) |
/// |             | `~/.config/powershell/Microsoft.PowerShell_profile.ps1` (Unix) |
///
/// # Errors
/// Returns [`RecallErrors::NoHomeDir`] if the home directory cannot be determined.
pub fn config_path(shell: &Shell) -> Result<PathBuf, RecallErrors> {
    let home = dirs::home_dir().ok_or(RecallErrors::NoHomeDir)?;

    let path = match shell {
        Shell::Bash => home.join(".bashrc"),
        Shell::Zsh => home.join(".zshrc"),

        // Fish uses a dedicated drop-in file in conf.d rather than modifying
        // the main config.fish. This is the idiomatic Fish convention and means
        // recall unhook can simply delete the file rather than editing it.
        Shell::Fish => home
            .join(".config")
            .join("fish")
            .join("conf.d")
            .join("recall.fish"),

        Shell::PowerShell => {
            if cfg!(windows) {
                home.join("Documents")
                    .join("PowerShell")
                    .join("Microsoft.PowerShell_profile.ps1")
            } else {
                home.join(".config")
                    .join("powershell")
                    .join("Microsoft.PowerShell_profile.ps1")
            }
        }
    };

    Ok(path)
}

// ── Hook scripts ──────────────────────────────────────────────────────────────

/// Returns the raw hook script for the given shell.
///
/// Scripts are embedded into the binary at compile time via `include_str!`
/// so the binary has no runtime dependency on the `hooks/` directory.
/// The content is the script body only — [`install_hook`] wraps it with
/// the [`HOOK_START`] and [`HOOK_END`] markers.
pub fn hook_script(shell: &Shell) -> &'static str {
    match shell {
        Shell::Bash => include_str!("../hooks/bash.sh"),
        Shell::Zsh => include_str!("../hooks/zsh.sh"),
        Shell::Fish => include_str!("../hooks/fish.fish"),
        Shell::PowerShell => include_str!("../hooks/powershell.ps1"),
    }
}

// ── Install / remove ──────────────────────────────────────────────────────────

/// Returns `true` if the recall hook block is already present in `content`.
///
/// Detection is based on the presence of [`HOOK_START`]. If the marker is
/// found, the hook is considered installed regardless of whether the end
/// marker is also present.
pub fn is_hook_installed(content: &str) -> bool {
    content.contains(HOOK_START)
}

/// Appends the hook block to `content` and returns the new string.
///
/// The script is wrapped between [`HOOK_START`] and [`HOOK_END`] markers so
/// that [`remove_hook`] can find and strip the exact block later. A single
/// blank line is inserted before the block to visually separate it from any
/// existing content in the config file.
///
/// # Example
///
/// Given an existing `.bashrc` and a bash hook script, the appended block
/// will look like:
///
/// ```text
/// # >>> recall hook start >>>
/// <script content>
/// # <<< recall hook end <<<
/// ```
pub fn install_hook(content: &str, script: &str) -> String {
    // Ensure exactly one blank line separates existing content from the block.
    let separator = if content.ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };

    format!(
        "{}{}{}\n{}\n{}\n",
        content,
        separator,
        HOOK_START,
        script.trim(),
        HOOK_END,
    )
}

/// Removes the hook block from `content` and returns the cleaned string.
///
/// All lines between and including [`HOOK_START`] and [`HOOK_END`] are
/// removed. Lines outside the block are preserved exactly. Any trailing
/// blank lines left behind by the removal are cleaned up and a single
/// trailing newline is restored.
///
/// Handles both Unix (`LF`) and Windows (`CRLF`) line endings by normalising
/// to `LF` before processing.
///
/// If the hook markers are not found the original content is returned
/// unchanged (trimmed to a single trailing newline).
pub fn remove_hook(content: &str) -> String {
    // Normalise CRLF → LF so the logic works identically on all platforms.
    let normalised = content.replace("\r\n", "\n");

    let mut result: Vec<&str> = Vec::new();
    let mut inside = false;

    for line in normalised.lines() {
        if line.trim() == HOOK_START {
            inside = true;
            continue;
        }
        if line.trim() == HOOK_END {
            inside = false;
            continue;
        }
        if !inside {
            result.push(line);
        }
    }

    // Clean up trailing blank lines left by the removed block, then
    // restore a single trailing newline so the file stays well-formed.
    let joined = result.join("\n");
    let trimmed = joined.trim_end();

    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{}\n", trimmed)
    }
}
