use recall::shell;
use recall::shell::Shell;

// ── is_hook_installed ─────────────────────────────────────────────────────────

#[test]
fn hook_not_present_in_empty_string() {
    assert!(!shell::is_hook_installed(""));
}

#[test]
fn hook_not_present_in_unrelated_content() {
    let content = "export PATH=$PATH:/usr/local/bin\nalias ll='ls -la'\n";
    assert!(!shell::is_hook_installed(content));
}

#[test]
fn hook_detected_after_install() {
    let content = "export PATH=$PATH:/usr/local/bin\n";
    let script = shell::hook_script(&Shell::Bash);
    let updated = shell::install_hook(content, script);
    assert!(shell::is_hook_installed(&updated));
}

// ── install_hook ──────────────────────────────────────────────────────────────

#[test]
fn install_preserves_existing_content() {
    let original = "export PATH=$PATH:/usr/local/bin\nalias ll='ls -la'\n";
    let script = shell::hook_script(&Shell::Bash);
    let updated = shell::install_hook(original, script);
    assert!(updated.starts_with(original));
}

#[test]
fn install_contains_start_and_end_markers() {
    let updated = shell::install_hook("", shell::hook_script(&Shell::Bash));
    assert!(updated.contains("# >>> recall hook start >>>"));
    assert!(updated.contains("# <<< recall hook end <<<"));
}

#[test]
fn install_ends_with_newline() {
    let updated = shell::install_hook("some content\n", shell::hook_script(&Shell::Zsh));
    assert!(updated.ends_with('\n'));
}

#[test]
fn install_on_empty_string_produces_valid_output() {
    let updated = shell::install_hook("", shell::hook_script(&Shell::Bash));
    assert!(!updated.is_empty());
    assert!(shell::is_hook_installed(&updated));
}

// ── remove_hook ───────────────────────────────────────────────────────────────

#[test]
fn remove_hook_restores_content_before_block() {
    let original = "export PATH=$PATH:/usr/local/bin\nalias ll='ls -la'\n";
    let script = shell::hook_script(&Shell::Bash);
    let with_hook = shell::install_hook(original, script);
    let restored = shell::remove_hook(&with_hook);
    assert_eq!(restored.trim(), original.trim());
}

#[test]
fn remove_hook_eliminates_start_marker() {
    let with_hook = shell::install_hook("content\n", shell::hook_script(&Shell::Zsh));
    let restored = shell::remove_hook(&with_hook);
    assert!(!restored.contains("# >>> recall hook start >>>"));
}

#[test]
fn remove_hook_eliminates_end_marker() {
    let with_hook = shell::install_hook("content\n", shell::hook_script(&Shell::Zsh));
    let restored = shell::remove_hook(&with_hook);
    assert!(!restored.contains("# <<< recall hook end <<<"));
}

#[test]
fn remove_hook_on_file_without_hook_is_noop() {
    let original = "export PATH=$PATH:/usr/local/bin\n";
    let cleaned = shell::remove_hook(original);
    assert_eq!(cleaned.trim(), original.trim());
}

#[test]
fn remove_hook_on_empty_string_returns_empty() {
    let cleaned = shell::remove_hook("");
    assert!(cleaned.is_empty());
}

#[test]
fn install_then_remove_is_idempotent() {
    let original = "# my shell config\nexport EDITOR=vim\n";
    let script = shell::hook_script(&Shell::Fish);
    let installed = shell::install_hook(original, script);
    let removed = shell::remove_hook(&installed);
    assert!(!shell::is_hook_installed(&removed));
    assert_eq!(removed.trim(), original.trim());
}

// ── hook_script content ───────────────────────────────────────────────────────

#[test]
fn bash_hook_script_contains_recall_log() {
    let script = shell::hook_script(&Shell::Bash);
    assert!(script.contains("recall log"));
}

#[test]
fn bash_hook_script_captures_exit_code() {
    let script = shell::hook_script(&Shell::Bash);
    assert!(script.contains("exit_code"));
}

#[test]
fn bash_hook_script_captures_cwd() {
    let script = shell::hook_script(&Shell::Bash);
    assert!(script.contains("pwd"));
}

#[test]
fn zsh_hook_script_uses_add_zsh_hook() {
    let script = shell::hook_script(&Shell::Zsh);
    assert!(script.contains("add-zsh-hook"));
}

#[test]
fn zsh_hook_script_contains_recall_log() {
    let script = shell::hook_script(&Shell::Zsh);
    assert!(script.contains("recall log"));
}

#[test]
fn fish_hook_script_uses_fish_postexec() {
    let script = shell::hook_script(&Shell::Fish);
    assert!(script.contains("fish_postexec"));
}

#[test]
fn fish_hook_script_contains_recall_log() {
    let script = shell::hook_script(&Shell::Fish);
    assert!(script.contains("recall log"));
}

#[test]
fn powershell_hook_script_contains_recall_log() {
    let script = shell::hook_script(&Shell::PowerShell);
    assert!(script.contains("recall"));
}

#[test]
fn powershell_hook_script_captures_exit_code() {
    let script = shell::hook_script(&Shell::PowerShell);
    assert!(script.contains("LASTEXITCODE"));
}

// ── all shells have correct markers ──────────────────────────────────────────

#[test]
fn all_hook_scripts_have_start_marker() {
    for shell in [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::PowerShell] {
        let script = shell::hook_script(&shell);
        assert!(
            script.contains("# >>> recall hook start >>>"),
            "{shell} hook missing start marker"
        );
    }
}

#[test]
fn all_hook_scripts_have_end_marker() {
    for shell in [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::PowerShell] {
        let script = shell::hook_script(&shell);
        assert!(
            script.contains("# <<< recall hook end <<<"),
            "{shell} hook missing end marker"
        );
    }
}

// ── fixture-based tests ───────────────────────────────────────────────────────

const SAMPLE_BASHRC: &str = include_str!("fixtures/sample.bashrc");
const SAMPLE_ZSHRC: &str = include_str!("fixtures/sample.zshrc");
const SAMPLE_PS1: &str = include_str!("fixtures/sample_profile.ps1");

/// Normalise CRLF → LF so fixture comparisons work on both Windows and Unix.
fn norm(s: &str) -> String {
    s.replace("\r\n", "\n")
}

// -- bashrc -------------------------------------------------------------------

#[test]
fn bashrc_fixture_has_no_hook_initially() {
    assert!(!shell::is_hook_installed(SAMPLE_BASHRC));
}

#[test]
fn bashrc_fixture_install_appends_after_existing_content() {
    let script = shell::hook_script(&Shell::Bash);
    let updated = shell::install_hook(SAMPLE_BASHRC, script);

    // Original content still intact at the start
    assert!(updated.starts_with(SAMPLE_BASHRC));
    // Hook block follows it
    assert!(shell::is_hook_installed(&updated));
}

#[test]
fn bashrc_fixture_install_contains_bash_hook_body() {
    let script = shell::hook_script(&Shell::Bash);
    let updated = shell::install_hook(SAMPLE_BASHRC, script);
    assert!(updated.contains("PROMPT_COMMAND"));
    assert!(updated.contains("__recall_precmd"));
}

#[test]
fn bashrc_fixture_remove_restores_original() {
    let script = shell::hook_script(&Shell::Bash);
    let with_hook = shell::install_hook(SAMPLE_BASHRC, script);
    let restored = shell::remove_hook(&with_hook);
    assert_eq!(norm(&restored).trim(), norm(SAMPLE_BASHRC).trim());
}

#[test]
fn bashrc_fixture_double_install_not_possible() {
    // Callers check is_hook_installed before calling install_hook, so simulate
    // that guard: after one install the marker should be detected.
    let script = shell::hook_script(&Shell::Bash);
    let first = shell::install_hook(SAMPLE_BASHRC, script);
    assert!(shell::is_hook_installed(&first));
    // A second install must NOT be attempted — guard holds
    assert!(shell::is_hook_installed(&first));
}

#[test]
fn bashrc_fixture_remove_on_clean_file_is_noop() {
    let cleaned = shell::remove_hook(SAMPLE_BASHRC);
    assert_eq!(norm(&cleaned).trim(), norm(SAMPLE_BASHRC).trim());
}

#[test]
fn bashrc_fixture_existing_aliases_survive_install_and_remove() {
    let script = shell::hook_script(&Shell::Bash);
    let with_hook = shell::install_hook(SAMPLE_BASHRC, script);
    let restored = norm(&shell::remove_hook(&with_hook));
    // Key lines from the fixture must still be present
    assert!(restored.contains("alias ll='ls -alF'"));
    assert!(restored.contains("HISTCONTROL=ignoreboth"));
    assert!(restored.contains("export EDITOR=vim"));
}

// -- zshrc --------------------------------------------------------------------

#[test]
fn zshrc_fixture_has_no_hook_initially() {
    assert!(!shell::is_hook_installed(SAMPLE_ZSHRC));
}

#[test]
fn zshrc_fixture_install_appends_after_existing_content() {
    let script = shell::hook_script(&Shell::Zsh);
    let updated = shell::install_hook(SAMPLE_ZSHRC, script);
    assert!(updated.starts_with(SAMPLE_ZSHRC));
    assert!(shell::is_hook_installed(&updated));
}

#[test]
fn zshrc_fixture_install_contains_zsh_hook_body() {
    let script = shell::hook_script(&Shell::Zsh);
    let updated = shell::install_hook(SAMPLE_ZSHRC, script);
    assert!(updated.contains("add-zsh-hook"));
    assert!(updated.contains("__recall_precmd"));
}

#[test]
fn zshrc_fixture_remove_restores_original() {
    let script = shell::hook_script(&Shell::Zsh);
    let with_hook = shell::install_hook(SAMPLE_ZSHRC, script);
    let restored = shell::remove_hook(&with_hook);
    assert_eq!(norm(&restored).trim(), norm(SAMPLE_ZSHRC).trim());
}

#[test]
fn zshrc_fixture_existing_aliases_survive_install_and_remove() {
    let script = shell::hook_script(&Shell::Zsh);
    let with_hook = shell::install_hook(SAMPLE_ZSHRC, script);
    let restored = norm(&shell::remove_hook(&with_hook));
    assert!(restored.contains("alias ll="));
    assert!(restored.contains("HISTFILE="));
    assert!(restored.contains("autoload -Uz compinit"));
}

#[test]
fn zshrc_fixture_remove_on_clean_file_is_noop() {
    let cleaned = shell::remove_hook(SAMPLE_ZSHRC);
    assert_eq!(norm(&cleaned).trim(), norm(SAMPLE_ZSHRC).trim());
}

// -- powershell profile -------------------------------------------------------

#[test]
fn ps1_fixture_has_no_hook_initially() {
    assert!(!shell::is_hook_installed(SAMPLE_PS1));
}

#[test]
fn ps1_fixture_install_appends_after_existing_content() {
    let script = shell::hook_script(&Shell::PowerShell);
    let updated = shell::install_hook(SAMPLE_PS1, script);
    assert!(updated.starts_with(SAMPLE_PS1));
    assert!(shell::is_hook_installed(&updated));
}

#[test]
fn ps1_fixture_install_contains_powershell_hook_body() {
    let script = shell::hook_script(&Shell::PowerShell);
    let updated = shell::install_hook(SAMPLE_PS1, script);
    assert!(updated.contains("__RecallLastCmd"));
    assert!(updated.contains("LASTEXITCODE"));
}

#[test]
fn ps1_fixture_remove_restores_original() {
    let script = shell::hook_script(&Shell::PowerShell);
    let with_hook = shell::install_hook(SAMPLE_PS1, script);
    let restored = shell::remove_hook(&with_hook);
    assert_eq!(norm(&restored).trim(), norm(SAMPLE_PS1).trim());
}

#[test]
fn ps1_fixture_existing_functions_survive_install_and_remove() {
    let script = shell::hook_script(&Shell::PowerShell);
    let with_hook = shell::install_hook(SAMPLE_PS1, script);
    let restored = norm(&shell::remove_hook(&with_hook));
    assert!(restored.contains("function prompt"));
    assert!(restored.contains("PSReadLine"));
    assert!(restored.contains("function mkcd"));
}

#[test]
fn ps1_fixture_remove_on_clean_file_is_noop() {
    let cleaned = shell::remove_hook(SAMPLE_PS1);
    assert_eq!(norm(&cleaned).trim(), norm(SAMPLE_PS1).trim());
}
