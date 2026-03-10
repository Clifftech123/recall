//! # Recall
//!
//! Entry point for the Recall CLI. Parses arguments with clap and dispatches
//! to the appropriate command handler in `src/commands/`.
//!
//! ## Error handling
//!
//! Most commands print an error to stderr and exit with code 1 on failure.
//! The `log` command is the exception — it runs silently in the background
//! after every shell command, so any error it produces is swallowed with
//! `.ok()` to ensure it never pollutes the user's terminal.

mod cli;
mod commands;
mod config;
mod db;
mod error;
mod format;
mod models;
mod shell;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    // Dispatch each subcommand to its handler and collect the Result.
    // The Log command is handled separately below — it must never error visibly.
    let result = match cli.command {
        Commands::Init => commands::init::run(),

        Commands::Hook => commands::hook::run(),

        Commands::Unhook => commands::unhook::run(),

        Commands::History {
            limit,
            json,
            csv,
            session,
            cwd,
            errors,
        } => commands::history::run(limit, json, csv, session, cwd, errors),

        Commands::Search {
            query,
            limit,
            json,
            csv,
        } => commands::search::run(&query, limit, json, csv),

        // Log is the hot path — runs after every shell command in the background.
        // Any error (database not initialised, disk full, etc.) is silently
        // discarded with .ok() so it never prints to the user's terminal.
        Commands::Log {
            command,
            session,
            cwd,
            exit_code,
            shell,
            metadata,
        } => {
            commands::log::run(command, session, cwd, exit_code, shell, metadata).ok();
            return;
        }

        Commands::Clean {
            all,
            from,
            to,
            dry_run,
        } => commands::clean::run(from.as_deref(), to.as_deref(), all, dry_run),

        Commands::Stats { top } => commands::stats::run(top),

        Commands::Undo => commands::undo::run(),

        Commands::Export { format, output } => {
            commands::export::run(format.as_deref(), output.as_deref())
        }

        Commands::Completions { shell } => commands::completions::run(shell),
    };

    // For all commands other than Log — print the error to stderr and exit
    // with a non-zero code so scripts and CI can detect failure.
    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
