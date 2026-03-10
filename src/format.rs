use crate::models::{Command, Stats};
use colored::Colorize;
use comfy_table::{Attribute, Cell, Color, Table};

/// Prints a formatted table of commands to stdout.
///
/// Each row shows the index, timestamp, exit code, shell, working directory,
/// and command text. Exit code is green for success (0) and red for failure.
/// Returns immediately with a dimmed message if the slice is empty.
pub fn as_table(commands: &[Command]) {
    if commands.is_empty() {
        println!("{}", "No commands found.".dimmed());
        return;
    }

    let mut table = Table::new();
    table.set_header(vec![
        Cell::new("#"),
        Cell::new("Time"),
        Cell::new("Exit"),
        Cell::new("Shell"),
        Cell::new("Directory"),
        Cell::new("Command"),
    ]);

    for (i, cmd) in commands.iter().enumerate() {
        // Show a green tick for success, red exit code number for failure.
        let exit_cell = if cmd.exit_code == 0 {
            Cell::new("✓").fg(Color::Green)
        } else {
            Cell::new(cmd.exit_code.to_string()).fg(Color::Red)
        };

        // Bold the command text so it stands out in the table.
        let command_cell = Cell::new(&cmd.command).add_attribute(Attribute::Bold);

        // Fall back gracefully when optional fields were not recorded.
        let cwd = cmd.display_cwd();
        let shell = cmd.shell.as_deref().unwrap_or("?");
        let time = cmd.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();

        table.add_row(vec![
            Cell::new(i + 1),
            Cell::new(time).fg(Color::DarkGrey),
            exit_cell,
            Cell::new(shell).fg(Color::Cyan),
            Cell::new(cwd).fg(Color::Blue),
            command_cell,
        ]);
    }

    println!("{table}");
}

/// Serialises `commands` to pretty-printed JSON and prints to stdout.
///
/// # Errors
/// Returns [`crate::error::RecallErrors::Json`] if serialisation fails.
pub fn as_json(commands: &[Command]) -> Result<(), crate::error::RecallErrors> {
    let json = serde_json::to_string_pretty(commands)?;
    println!("{}", json);
    Ok(())
}

/// Writes `commands` as CSV to stdout.
///
/// The header row is always written first. Optional fields (`session_id`,
/// `cwd`, `shell`, `hostname`) are written as empty strings when absent.
///
/// # Errors
/// Returns [`crate::error::RecallErrors::Csv`] if any write or flush fails.
pub fn as_csv(commands: &[Command]) -> Result<(), crate::error::RecallErrors> {
    let mut writer = csv::Writer::from_writer(std::io::stdout());

    // Write the header row.
    writer.write_record(&[
        "id",
        "command",
        "timestamp",
        "session_id",
        "cwd",
        "exit_code",
        "shell",
        "hostname",
    ])?;

    for cmd in commands {
        writer.write_record(&[
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

    // Flush ensures the final record is written even if stdout is buffered.
    writer.flush()?;
    Ok(())
}

/// Prints a usage statistics dashboard to stdout.
///
/// Displays total and unique command counts, date range, error rate, a
/// ranked bar chart of the most-used commands, top directories, and a
/// busiest-hours chart. Sections are skipped silently if their data is empty.
pub fn as_stats(stats: &Stats) {
    println!("{}", "\n  Recall Statistics".bold());
    println!("  {}", "═════════════════".dimmed());

    println!(
        "\n  Total commands:   {}",
        stats.total_commands.to_string().bold()
    );
    println!(
        "  Unique commands:  {}",
        stats.unique_commands.to_string().bold()
    );
    println!(
        "  Date range:       {} → {}",
        stats.date_range.0.dimmed(),
        stats.date_range.1.dimmed()
    );
    println!(
        "  Error rate:       {}",
        format!("{:.1}%", stats.error_rate).red()
    );

    // ── Top commands ──────────────────────────────────────────────────────────

    if !stats.top_commands.is_empty() {
        println!("\n  {}", "Top Commands".bold());
        println!("  {}", "────────────".dimmed());

        // The first entry has the highest count — use it to scale all bars.
        let max_count = stats.top_commands[0].1;

        for (i, (cmd, count)) in stats.top_commands.iter().enumerate() {
            let bar = render_bar(*count, max_count, 20);
            let pct = (*count as f64 / stats.total_commands as f64) * 100.0;
            println!(
                "  {:>2}.  {:<35} {:>6}  {}  {:.1}%",
                i + 1,
                cmd.cyan(),
                count.to_string().bold(),
                bar,
                pct
            );
        }
    }

    // ── Top directories ───────────────────────────────────────────────────────

    if !stats.top_directories.is_empty() {
        println!("\n  {}", "Top Directories".bold());
        println!("  {}", "───────────────".dimmed());

        for (i, (dir, count)) in stats.top_directories.iter().enumerate() {
            println!(
                "  {:>2}.  {:<40} {}",
                i + 1,
                dir.blue(),
                count.to_string().bold()
            );
        }
    }

    // ── Busiest hours ─────────────────────────────────────────────────────────

    if !stats.most_active_hours.is_empty() {
        println!("\n  {}", "Busiest Hours".bold());
        println!("  {}", "─────────────".dimmed());

        // Scale bars relative to the busiest single hour.
        let max_count = stats
            .most_active_hours
            .iter()
            .map(|(_, c)| *c)
            .max()
            .unwrap_or(1);

        for (hour, count) in &stats.most_active_hours {
            let bar = render_bar(*count, max_count, 16);
            println!("  {:02}:00  {}  {}", hour, bar, count.to_string().dimmed());
        }
    }

    println!();
}

/// Renders a Unicode block bar scaled to `width` characters.
///
/// Filled characters (`█`) represent the proportion of `value` relative to
/// `max`. Empty characters (`░`) fill the remainder. Both are coloured —
/// filled green, empty dimmed — for visual clarity.
///
/// # Example
///
/// `render_bar(3, 10, 10)` → `"███░░░░░░░"` (3 filled, 7 empty)
fn render_bar(value: u64, max: u64, width: usize) -> String {
    let filled = ((value as f64 / max as f64) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!(
        "{}{}",
        "█".repeat(filled).green(),
        "░".repeat(empty).dimmed()
    )
}
