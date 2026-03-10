use clap::{Parser, Subcommand};

/// The top-level CLI struct.
///
/// Parsed by clap from the command-line arguments. Every subcommand is
/// dispatched from `main.rs` based on the [`Commands`] variant matched.
#[derive(Parser)]
#[command(name = "recall")]
#[command(about = "Cross-platform command history that never forgets")]
#[command(long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// All subcommands Recall supports.
///
/// Each variant maps directly to a handler in `src/commands/`.
#[derive(Subcommand)]
pub enum Commands {
    /// Initialise the database and create ~/.recall/
    Init,

    /// Install the logging hook into your shell config
    Hook,

    /// Remove the logging hook from your shell config
    Unhook,

    /// View command history
    History {
        /// Maximum number of entries to return
        #[arg(long, default_value_t = 100)]
        limit: u64,

        /// Output as a JSON array
        #[arg(long, conflicts_with = "csv")]
        json: bool,

        /// Output as CSV
        #[arg(long, conflicts_with = "json")]
        csv: bool,

        /// Filter results to a specific shell session ID
        #[arg(long)]
        session: Option<String>,

        /// Filter results to a specific working directory
        #[arg(long)]
        cwd: Option<String>,

        /// Show only commands that exited with a non-zero exit code
        #[arg(long)]
        errors: bool,
    },

    /// Search command history using full-text search
    Search {
        /// The search query — supports FTS5 syntax (e.g. "git*", "git AND commit")
        query: String,

        /// Maximum number of results to return
        #[arg(long, default_value_t = 10)]
        limit: u64,

        /// Output as a JSON array
        #[arg(long, conflicts_with = "csv")]
        json: bool,

        /// Output as CSV
        #[arg(long, conflicts_with = "json")]
        csv: bool,
    },

    /// Log a command manually (used internally by shell hooks)
    Log {
        /// The command string to log
        command: String,

        /// Shell session ID (defaults to current shell PID)
        #[arg(long)]
        session: Option<String>,

        /// Working directory the command was run in
        #[arg(long)]
        cwd: Option<String>,

        /// Exit code of the command
        #[arg(long, default_value_t = 0)]
        exit_code: i32,

        /// Shell the command was run in (bash, zsh, fish, pwsh)
        #[arg(long)]
        shell: Option<String>,

        /// Optional JSON metadata blob for extra context
        #[arg(long)]
        metadata: Option<String>,
    },

    /// Delete history by date range or wipe everything
    Clean {
        /// Delete all history — cannot be combined with --from or --to
        #[arg(long, conflicts_with_all = ["from", "to"])]
        all: bool,

        /// Delete entries on or after this date (YYYY-MM-DD or RFC3339)
        #[arg(long)]
        from: Option<String>,

        /// Delete entries on or before this date (YYYY-MM-DD or RFC3339)
        #[arg(long)]
        to: Option<String>,

        /// Preview how many entries would be deleted without actually deleting
        #[arg(long)]
        dry_run: bool,
    },

    /// Show usage statistics — top commands, busiest hours, error rate
    Stats {
        /// How many top entries to show in ranked lists
        #[arg(long, default_value_t = 10)]
        top: u64,
    },

    /// Remove the last logged command
    ///
    /// Use this immediately if you accidentally typed a password or secret
    /// as a command — it removes the entry from the database before it can
    /// be searched or exported.
    Undo,

    /// Export full history to a file or stdout
    Export {
        /// Output format: json, csv, or text (default: json)
        #[arg(long)]
        format: Option<String>,

        /// Output file path — prints to stdout if not specified
        #[arg(long)]
        output: Option<String>,
    },

    /// Generate shell completion scripts
    Completions {
        /// The shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}
