use thiserror::Error;
#[derive(Error, Debug)]
pub enum RecallErrors {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    #[error("Cannot find home directory. Set $HOME and try again.")]
    NoHomeDir,

    #[error("Shell not detected. Use --shell flag or set $SHELL.")]
    UnknownShell,

    #[error("Hook already installed in {0}. Run 'recall unhook' first.")]
    HookAlreadyInstalled(String),

    #[error("Cannot read shell config: {path}\n  Reason: {reason}")]
    ShellConfigRead { path: String, reason: String },

    #[error("Cannot write shell config: {path}\n  Reason: {reason}\n  Backup at: {backup}")]
    ShellConfigWrite {
        path: String,
        reason: String,
        backup: String,
    },

    #[error("Invalid date format: '{input}'. Use YYYY-MM-DD or RFC3339.")]
    InvalidDate { input: String },

    #[error("Cannot use --all with --from or --to. Choose one approach.")]
    ConflictingCleanFlags,
}
