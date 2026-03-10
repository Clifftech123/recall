use crate::error::RecallErrors;
use crate::models::{Command, Stats};
use rusqlite::Connection;
use std::path::PathBuf;

/// The single database connection wrapper for Recall.
///
/// Every SQLite operation in the application goes through this struct.
/// Create one instance with [`Database::open`] and pass it down to
/// whichever command handler needs it.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Opens (or creates) the SQLite database at `path` and prepares it for use.
    ///
    /// Steps performed in order:
    /// 1. Open the `.db` file — created automatically if it does not exist
    /// 2. Apply performance pragmas (WAL mode, 8 MB cache, etc.)
    /// 3. Run [`Database::migrate`] to ensure the schema is current
    ///
    /// # Errors
    /// Returns [`RecallErrors::Database`] if the file cannot be opened,
    /// pragmas fail, or migration fails.
    pub fn open(path: &PathBuf) -> Result<Self, RecallErrors> {
        let conn = Connection::open(path)?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous  = NORMAL;
             PRAGMA cache_size   = -8000;
             PRAGMA foreign_keys = ON;",
        )?;

        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    /// Creates all tables, virtual FTS5 table, triggers, and indexes
    /// if they do not already exist.
    ///
    /// Safe to call on an already-initialised database — every statement
    /// uses `CREATE ... IF NOT EXISTS` so nothing is dropped or overwritten.
    ///
    /// Schema overview:
    /// - `commands` — one row per logged command
    /// - `commands_fts` — FTS5 content table that indexes `command` and `cwd`
    /// - `commands_ai` — trigger that keeps the FTS index in sync on INSERT
    /// - `commands_ad` — trigger that keeps the FTS index in sync on DELETE
    /// - Three indexes on `timestamp`, `session_id`, and `exit_code`
    ///
    /// # Errors
    /// Returns [`RecallErrors::Database`] if any statement fails.
    fn migrate(&self) -> Result<(), RecallErrors> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS commands (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                command     TEXT    NOT NULL,
                timestamp   TEXT    NOT NULL,
                session_id  TEXT,
                cwd         TEXT,
                exit_code   INTEGER DEFAULT 0,
                shell       TEXT,
                hostname    TEXT,
                metadata    TEXT
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS commands_fts USING fts5(
                command,
                cwd,
                content='commands',
                content_rowid='id'
            );

            CREATE TRIGGER IF NOT EXISTS commands_ai AFTER INSERT ON commands BEGIN
                INSERT INTO commands_fts(rowid, command, cwd)
                VALUES (new.id, new.command, new.cwd);
            END;

            CREATE TRIGGER IF NOT EXISTS commands_ad AFTER DELETE ON commands BEGIN
                INSERT INTO commands_fts(commands_fts, rowid, command, cwd)
                VALUES ('delete', old.id, old.command, old.cwd);
            END;

            CREATE INDEX IF NOT EXISTS idx_commands_timestamp ON commands(timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_commands_session    ON commands(session_id);
            CREATE INDEX IF NOT EXISTS idx_commands_exit_code  ON commands(exit_code);",
        )?;
        Ok(())
    }

    /// Inserts a new command entry into the database and returns its row ID.
    ///
    /// The FTS5 `commands_ai` trigger fires automatically after the insert,
    /// so the full-text index is updated without any extra work here.
    ///
    /// # Errors
    /// Returns [`RecallErrors::Database`] if the insert fails.
    pub fn insert(&self, command: &Command) -> Result<i64, RecallErrors> {
        self.conn.execute(
            "INSERT INTO commands
                (command, timestamp, session_id, cwd, exit_code, shell, hostname, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                command.command,
                command.timestamp.to_rfc3339(),
                command.session_id,
                command.cwd,
                command.exit_code,
                command.shell,
                command.hostname,
                command.metadata,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Returns recent commands ordered by timestamp descending.
    ///
    /// All filter arguments are optional — pass `None` to skip a filter.
    ///
    /// - `limit`       — maximum number of rows to return
    /// - `session`     — restrict to a specific shell session ID
    /// - `cwd`         — restrict to a specific working directory
    /// - `errors_only` — when `true`, only return commands where `exit_code != 0`
    ///
    /// # Errors
    /// Returns [`RecallErrors::Database`] if the query fails.
    pub fn history(
        &self,
        limit: u64,
        session: Option<&str>,
        cwd: Option<&str>,
        errors_only: bool,
    ) -> Result<Vec<Command>, RecallErrors> {
        // Build the WHERE clause dynamically based on which filters are active.
        // ?1 is always reserved for the LIMIT value.
        let mut sql = String::from(
            "SELECT id, command, timestamp, session_id, cwd, exit_code, shell, hostname, metadata
             FROM commands
             WHERE 1=1",
        );

        // Collect boxed params so we can push them in order.
        // ?1 = limit, subsequent slots filled as filters are added.
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(limit as i64)];
        let mut idx = 2usize;

        if let Some(s) = session {
            sql.push_str(&format!(" AND session_id = ?{idx}"));
            params.push(Box::new(s.to_string()));
            idx += 1;
        }

        if let Some(c) = cwd {
            sql.push_str(&format!(" AND cwd = ?{idx}"));
            params.push(Box::new(c.to_string()));
            idx += 1;
        }

        if errors_only {
            sql.push_str(" AND exit_code != 0");
        }

        // idx is intentionally unused after the last conditional increment.
        let _ = idx;

        sql.push_str(" ORDER BY timestamp DESC LIMIT ?1");

        let mut stmt = self.conn.prepare(&sql)?;
        let refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(refs.as_slice(), Self::row_to_command)?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(RecallErrors::Database)
    }

    /// Searches command history using the FTS5 full-text index.
    ///
    /// The `query` string supports FTS5 syntax:
    /// - `"git commit"` — exact phrase
    /// - `git*`         — prefix match
    /// - `git AND NOT merge` — boolean logic
    ///
    /// Results are returned ordered by FTS5 relevance rank (best match first).
    ///
    /// # Errors
    /// Returns [`RecallErrors::Database`] if the query fails.
    pub fn search(&self, query: &str, limit: u64) -> Result<Vec<Command>, RecallErrors> {
        let mut stmt = self.conn.prepare(
            "SELECT c.id, c.command, c.timestamp, c.session_id, c.cwd,
                    c.exit_code, c.shell, c.hostname, c.metadata
             FROM commands c
             JOIN commands_fts f ON c.id = f.rowid
             WHERE commands_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(rusqlite::params![query, limit as i64], Self::row_to_command)?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(RecallErrors::Database)
    }

    /// Removes the most recently logged command and returns it.
    ///
    /// Returns `None` if the database is empty.
    /// Used by `recall undo` to let the user quickly remove the last entry
    /// — for example if a password was accidentally typed as a command.
    ///
    /// # Errors
    /// Returns [`RecallErrors::Database`] if the delete fails.
    ///
    /// Bug fix: previous version accepted a `command_id` parameter that was
    /// never used — the method always deleted the last row regardless. The
    /// parameter has been removed so the signature matches the actual behaviour.
    pub fn undo(&self) -> Result<Option<Command>, RecallErrors> {
        // Fetch the last row first so we can return it to the caller.
        let cmd = self
            .conn
            .query_row(
                "SELECT id, command, timestamp, session_id, cwd,
                         exit_code, shell, hostname, metadata
                  FROM commands ORDER BY id DESC LIMIT 1",
                [],
                Self::row_to_command,
            )
            .ok();

        if let Some(ref c) = cmd {
            self.conn.execute(
                "DELETE FROM commands WHERE id = ?1",
                rusqlite::params![c.id],
            )?;
        }

        Ok(cmd)
    }

    /// Deletes history entries by date range or all at once.
    ///
    /// - `all`     — delete every row (cannot be combined with `from`/`to`)
    /// - `from`    — delete entries on or after this date (ISO 8601)
    /// - `to`      — delete entries on or before this date (ISO 8601)
    /// - `dry_run` — return the count of rows that *would* be deleted without
    ///               actually deleting anything
    ///
    /// Returns the number of rows deleted (or that would be deleted on dry run).
    ///
    /// # Errors
    /// Returns [`RecallErrors::Database`] if any query fails.
    pub fn clean(
        &self,
        from: Option<&str>,
        to: Option<&str>,
        all: bool,
        dry_run: bool,
    ) -> Result<u64, RecallErrors> {
        if all {
            if dry_run {
                let count: i64 = self
                    .conn
                    .query_row("SELECT COUNT(*) FROM commands", [], |r| r.get(0))?;
                return Ok(count as u64);
            }
            let count = self.conn.execute("DELETE FROM commands", [])?;
            return Ok(count as u64);
        }

        // Build the WHERE clause based on which date bounds were supplied.
        let (where_clause, count_sql, delete_sql) = match (from, to) {
            (Some(_), Some(_)) => (
                "timestamp BETWEEN ?1 AND ?2",
                "SELECT COUNT(*) FROM commands WHERE timestamp BETWEEN ?1 AND ?2",
                "DELETE FROM commands WHERE timestamp BETWEEN ?1 AND ?2",
            ),
            (Some(_), None) => (
                "timestamp >= ?1",
                "SELECT COUNT(*) FROM commands WHERE timestamp >= ?1",
                "DELETE FROM commands WHERE timestamp >= ?1",
            ),
            (None, Some(_)) => (
                "timestamp <= ?2",
                "SELECT COUNT(*) FROM commands WHERE timestamp <= ?2",
                "DELETE FROM commands WHERE timestamp <= ?2",
            ),
            // Nothing to do if neither bound was supplied.
            (None, None) => return Ok(0),
        };

        let _ = where_clause; // used only to document intent above

        if dry_run {
            let count: i64 = self
                .conn
                .query_row(count_sql, rusqlite::params![from, to], |r| r.get(0))?;
            return Ok(count as u64);
        }

        let count = self.conn.execute(delete_sql, rusqlite::params![from, to])?;
        Ok(count as u64)
    }

    /// Returns all commands ordered by timestamp ascending.
    ///
    /// Used by `recall export` to dump the full history to JSON, CSV, or text.
    /// Ascending order is used so the exported file reads chronologically.
    ///
    /// # Errors
    /// Returns [`RecallErrors::Database`] if the query fails.
    pub fn export(&self) -> Result<Vec<Command>, RecallErrors> {
        let mut stmt = self.conn.prepare(
            "SELECT id, command, timestamp, session_id, cwd,
                    exit_code, shell, hostname, metadata
             FROM commands
             ORDER BY timestamp ASC",
        )?;

        let rows = stmt.query_map([], Self::row_to_command)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(RecallErrors::Database)
    }

    /// Aggregates usage data across all stored commands and returns a [`Stats`] summary.
    ///
    /// `top` controls how many entries appear in `top_commands` and `top_directories`.
    /// The heavy lifting is delegated to private helper methods, each running a single
    /// focused query, to keep this method readable.
    ///
    /// # Errors
    /// Returns [`RecallErrors::Database`] if any aggregation query fails.
    pub fn stats(&self, top: u64) -> Result<Stats, RecallErrors> {
        let total_commands = self.stats_total();
        let unique_commands = self.stats_unique();
        let date_range = self.stats_date_range();
        let error_rate = self.stats_error_rate(total_commands);
        let top_commands = self.stats_top_commands(top)?;
        let top_directories = self.stats_top_directories(top)?;
        let most_active_hours = self.stats_active_hours()?;

        Ok(Stats {
            total_commands,
            unique_commands,
            date_range,
            top_commands,
            most_active_hours,
            error_rate,
            top_directories,
        })
    }

    // -------------------------------------------------------------------------
    // Private stats helpers — each runs one focused aggregation query.
    // Returning 0 / default on failure keeps `stats()` non-fatal for display.
    // -------------------------------------------------------------------------

    /// Total number of rows in the commands table.
    fn stats_total(&self) -> u64 {
        self.conn
            .query_row("SELECT COUNT(*) FROM commands", [], |r| r.get(0))
            .unwrap_or(0)
    }

    /// Number of distinct command strings ever logged.
    fn stats_unique(&self) -> u64 {
        self.conn
            .query_row("SELECT COUNT(DISTINCT command) FROM commands", [], |r| {
                r.get(0)
            })
            .unwrap_or(0)
    }

    /// Earliest and latest timestamps formatted as YYYY-MM-DD.
    fn stats_date_range(&self) -> (String, String) {
        self.conn
            .query_row(
                "SELECT
                    COALESCE(strftime('%Y-%m-%d', MIN(timestamp)), ''),
                    COALESCE(strftime('%Y-%m-%d', MAX(timestamp)), '')
                 FROM commands",
                [],
                |r| {
                    Ok((
                        r.get::<_, String>(0).unwrap_or_default(),
                        r.get::<_, String>(1).unwrap_or_default(),
                    ))
                },
            )
            .unwrap_or_default()
    }

    /// Percentage of commands that exited with a non-zero exit code.
    fn stats_error_rate(&self, total: u64) -> f64 {
        if total == 0 {
            return 0.0;
        }
        let errors: u64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM commands WHERE exit_code != 0",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        (errors as f64 / total as f64) * 100.0
    }

    /// The `top` most frequently run commands with their counts.
    fn stats_top_commands(&self, top: u64) -> Result<Vec<(String, u64)>, RecallErrors> {
        let mut stmt = self.conn.prepare(
            "SELECT command, COUNT(*) as cnt
             FROM commands
             GROUP BY command
             ORDER BY cnt DESC
             LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![top as i64], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, u64>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(RecallErrors::Database);
        rows
    }

    /// The `top` directories where the most commands were run.
    fn stats_top_directories(&self, top: u64) -> Result<Vec<(String, u64)>, RecallErrors> {
        let mut stmt = self.conn.prepare(
            "SELECT cwd, COUNT(*) as cnt
             FROM commands
             WHERE cwd IS NOT NULL
             GROUP BY cwd
             ORDER BY cnt DESC
             LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![top as i64], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, u64>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(RecallErrors::Database);
        rows
    }

    /// Command counts grouped by hour of the day (0–23).
    fn stats_active_hours(&self) -> Result<Vec<(u8, u64)>, RecallErrors> {
        let mut stmt = self.conn.prepare(
            "SELECT CAST(strftime('%H', timestamp) AS INTEGER) as hour, COUNT(*) as cnt
             FROM commands
             GROUP BY hour
             ORDER BY hour ASC",
        )?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, u8>(0)?, r.get::<_, u64>(1)?)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(RecallErrors::Database);
        rows
    }

    // -------------------------------------------------------------------------
    // Shared helpers
    // -------------------------------------------------------------------------

    /// Maps a single SQLite row to a [`Command`] struct.
    ///
    /// Column order must match every SELECT in this file:
    /// `id, command, timestamp, session_id, cwd, exit_code, shell, hostname, metadata`
    ///
    /// The timestamp is stored as an RFC 3339 string and parsed back here.
    /// Falls back to the Unix epoch if parsing fails rather than returning an error.
    pub fn row_to_command(row: &rusqlite::Row) -> Result<Command, rusqlite::Error> {
        let ts: String = row.get(2)?;
        Ok(Command {
            id: row.get(0)?,
            command: row.get(1)?,
            timestamp: ts.parse().unwrap_or_default(),
            session_id: row.get(3)?,
            cwd: row.get(4)?,
            exit_code: row.get(5)?,
            shell: row.get(6)?,
            hostname: row.get(7)?,
            metadata: row.get(8)?,
        })
    }
}
