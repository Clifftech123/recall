use crate::cli::Cli;
use crate::error::RecallErrors;
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

/// Entry point for `recall completions`.
///
/// Generates a shell completion script for the given shell and prints it
/// to stdout. The output should be piped to the appropriate location for
/// the user's shell:
///
/// ```sh
/// # bash
/// recall completions bash > ~/.local/share/bash-completion/completions/recall
///
/// # zsh
/// recall completions zsh > ~/.zfunc/_recall
///
/// # fish
/// recall completions fish > ~/.config/fish/completions/recall.fish
///
/// # PowerShell
/// recall completions powershell >> $PROFILE
/// ```
///
/// Completions are generated at runtime from the live [`Cli`] definition,
/// so they are always in sync with the current set of subcommands and flags —
/// no manual maintenance required.
///
/// # Errors
/// This function is infallible in practice — `generate` writes directly to
/// stdout and clap handles all formatting internally. The `Result` return
/// type exists for consistency with all other command handlers.
pub fn run(shell: Shell) -> Result<(), RecallErrors> {
    let mut cmd = Cli::command();

    // get_name() returns the binary name defined in Cargo.toml ("recall").
    // This is passed to generate() so the completion script references the
    // correct binary name rather than a hardcoded string.
    let bin_name = cmd.get_name().to_string();

    // generate() writes the completion script directly to stdout.
    // clap_complete handles all shell-specific formatting internally.
    generate(shell, &mut cmd, bin_name, &mut io::stdout());

    Ok(())
}
