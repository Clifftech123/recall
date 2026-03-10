use chrono::Utc;
use recall::db::Database;
use recall::models::Command;
use std::path::PathBuf;

// ── helpers ───────────────────────────────────────────────────────────────────

fn in_memory_db() -> Database {
    Database::open(&PathBuf::from(":memory:")).expect("failed to open in-memory db")
}

fn make_cmd(command: &str, exit_code: i32, cwd: &str, shell: &str, session: &str) -> Command {
    Command {
        id: 0,
        command: command.to_string(),
        timestamp: Utc::now(),
        session_id: Some(session.to_string()),
        cwd: Some(cwd.to_string()),
        exit_code,
        shell: Some(shell.to_string()),
        hostname: Some("testhost".to_string()),
        metadata: None,
    }
}

// ── insert ────────────────────────────────────────────────────────────────────

#[test]
fn insert_returns_incrementing_ids() {
    let db = in_memory_db();

    let id1 = db
        .insert(&make_cmd("git status", 0, "/home", "bash", "1"))
        .unwrap();
    let id2 = db
        .insert(&make_cmd("cargo build", 0, "/home", "bash", "1"))
        .unwrap();

    assert!(id1 > 0);
    assert_eq!(id2, id1 + 1);
}

#[test]
fn insert_stores_all_fields() {
    let db = in_memory_db();

    db.insert(&make_cmd("ls -la", 0, "/tmp", "zsh", "42"))
        .unwrap();

    let rows = db.history(1, None, None, false).unwrap();
    assert_eq!(rows.len(), 1);

    let cmd = &rows[0];
    assert_eq!(cmd.command, "ls -la");
    assert_eq!(cmd.exit_code, 0);
    assert_eq!(cmd.cwd.as_deref(), Some("/tmp"));
    assert_eq!(cmd.shell.as_deref(), Some("zsh"));
    assert_eq!(cmd.session_id.as_deref(), Some("42"));
    assert_eq!(cmd.hostname.as_deref(), Some("testhost"));
}

// ── history ───────────────────────────────────────────────────────────────────

#[test]
fn history_respects_limit() {
    let db = in_memory_db();

    for i in 0..10 {
        db.insert(&make_cmd(&format!("cmd {}", i), 0, "/home", "bash", "1"))
            .unwrap();
    }

    let rows = db.history(3, None, None, false).unwrap();
    assert_eq!(rows.len(), 3);
}

#[test]
fn history_orders_newest_first() {
    let db = in_memory_db();

    db.insert(&make_cmd("first", 0, "/home", "bash", "1"))
        .unwrap();
    db.insert(&make_cmd("second", 0, "/home", "bash", "1"))
        .unwrap();
    db.insert(&make_cmd("third", 0, "/home", "bash", "1"))
        .unwrap();

    let rows = db.history(10, None, None, false).unwrap();
    assert_eq!(rows[0].command, "third");
    assert_eq!(rows[2].command, "first");
}

#[test]
fn history_filters_by_session() {
    let db = in_memory_db();

    db.insert(&make_cmd("session-a-cmd", 0, "/home", "bash", "session-a"))
        .unwrap();
    db.insert(&make_cmd("session-b-cmd", 0, "/home", "bash", "session-b"))
        .unwrap();

    let rows = db.history(10, Some("session-a"), None, false).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].command, "session-a-cmd");
}

#[test]
fn history_filters_by_cwd() {
    let db = in_memory_db();

    db.insert(&make_cmd("cmd-in-projects", 0, "/projects", "bash", "1"))
        .unwrap();
    db.insert(&make_cmd("cmd-in-home", 0, "/home", "bash", "1"))
        .unwrap();

    let rows = db.history(10, None, Some("/projects"), false).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].command, "cmd-in-projects");
}

#[test]
fn history_errors_only_filters_non_zero_exit() {
    let db = in_memory_db();

    db.insert(&make_cmd("good", 0, "/home", "bash", "1"))
        .unwrap();
    db.insert(&make_cmd("bad", 1, "/home", "bash", "1"))
        .unwrap();
    db.insert(&make_cmd("also-bad", 127, "/home", "bash", "1"))
        .unwrap();

    let rows = db.history(10, None, None, true).unwrap();
    assert_eq!(rows.len(), 2);
    for cmd in &rows {
        assert_ne!(cmd.exit_code, 0);
    }
}

#[test]
fn history_empty_db_returns_empty_vec() {
    let db = in_memory_db();
    let rows = db.history(10, None, None, false).unwrap();
    assert!(rows.is_empty());
}

// ── search ────────────────────────────────────────────────────────────────────

#[test]
fn search_finds_exact_word() {
    let db = in_memory_db();

    db.insert(&make_cmd("git commit -m fix", 0, "/home", "bash", "1"))
        .unwrap();
    db.insert(&make_cmd("cargo test", 0, "/home", "bash", "1"))
        .unwrap();
    db.insert(&make_cmd("git push origin main", 0, "/home", "bash", "1"))
        .unwrap();

    let results = db.search("cargo", 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].command, "cargo test");
}

#[test]
fn search_returns_multiple_matches() {
    let db = in_memory_db();

    db.insert(&make_cmd("git status", 0, "/home", "bash", "1"))
        .unwrap();
    db.insert(&make_cmd("git commit -m fix", 0, "/home", "bash", "1"))
        .unwrap();
    db.insert(&make_cmd("git push origin main", 0, "/home", "bash", "1"))
        .unwrap();
    db.insert(&make_cmd("cargo build", 0, "/home", "bash", "1"))
        .unwrap();

    let results = db.search("git", 10).unwrap();
    assert_eq!(results.len(), 3);
}

#[test]
fn search_respects_limit() {
    let db = in_memory_db();

    for i in 0..5 {
        db.insert(&make_cmd(
            &format!("docker run image-{}", i),
            0,
            "/home",
            "bash",
            "1",
        ))
        .unwrap();
    }

    let results = db.search("docker", 2).unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn search_no_match_returns_empty() {
    let db = in_memory_db();

    db.insert(&make_cmd("git status", 0, "/home", "bash", "1"))
        .unwrap();

    let results = db.search("kubernetes", 10).unwrap();
    assert!(results.is_empty());
}

#[test]
fn search_empty_db_returns_empty() {
    let db = in_memory_db();
    let results = db.search("anything", 10).unwrap();
    assert!(results.is_empty());
}

// ── undo ──────────────────────────────────────────────────────────────────────

#[test]
fn undo_removes_last_command() {
    let db = in_memory_db();

    db.insert(&make_cmd("first", 0, "/home", "bash", "1"))
        .unwrap();
    db.insert(&make_cmd("second", 0, "/home", "bash", "1"))
        .unwrap();
    db.insert(&make_cmd("third", 0, "/home", "bash", "1"))
        .unwrap();

    let removed = db.undo().unwrap();
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().command, "third");

    let remaining = db.history(10, None, None, false).unwrap();
    assert_eq!(remaining.len(), 2);
    assert_eq!(remaining[0].command, "second");
}

#[test]
fn undo_on_empty_db_returns_none() {
    let db = in_memory_db();
    let result = db.undo().unwrap();
    assert!(result.is_none());
}

#[test]
fn undo_twice_removes_two_commands() {
    let db = in_memory_db();

    db.insert(&make_cmd("a", 0, "/home", "bash", "1")).unwrap();
    db.insert(&make_cmd("b", 0, "/home", "bash", "1")).unwrap();
    db.insert(&make_cmd("c", 0, "/home", "bash", "1")).unwrap();

    db.undo().unwrap();
    db.undo().unwrap();

    let remaining = db.history(10, None, None, false).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].command, "a");
}

// ── clean ─────────────────────────────────────────────────────────────────────

#[test]
fn clean_all_deletes_everything() {
    let db = in_memory_db();

    for i in 0..5 {
        db.insert(&make_cmd(&format!("cmd {}", i), 0, "/home", "bash", "1"))
            .unwrap();
    }

    let deleted = db.clean(None, None, true, false).unwrap();
    assert_eq!(deleted, 5);

    let remaining = db.history(100, None, None, false).unwrap();
    assert!(remaining.is_empty());
}

#[test]
fn clean_all_dry_run_does_not_delete() {
    let db = in_memory_db();

    for i in 0..5 {
        db.insert(&make_cmd(&format!("cmd {}", i), 0, "/home", "bash", "1"))
            .unwrap();
    }

    let would_delete = db.clean(None, None, true, true).unwrap();
    assert_eq!(would_delete, 5);

    // Nothing actually deleted
    let remaining = db.history(100, None, None, false).unwrap();
    assert_eq!(remaining.len(), 5);
}

#[test]
fn clean_no_args_returns_zero() {
    let db = in_memory_db();
    db.insert(&make_cmd("cmd", 0, "/home", "bash", "1"))
        .unwrap();

    let deleted = db.clean(None, None, false, false).unwrap();
    assert_eq!(deleted, 0);
}

// ── export ────────────────────────────────────────────────────────────────────

#[test]
fn export_returns_all_commands_oldest_first() {
    let db = in_memory_db();

    db.insert(&make_cmd("first", 0, "/home", "bash", "1"))
        .unwrap();
    db.insert(&make_cmd("second", 0, "/home", "bash", "1"))
        .unwrap();
    db.insert(&make_cmd("third", 0, "/home", "bash", "1"))
        .unwrap();

    let rows = db.export().unwrap();
    assert_eq!(rows.len(), 3);
    // export orders ASC — oldest first
    assert_eq!(rows[0].command, "first");
    assert_eq!(rows[2].command, "third");
}

#[test]
fn export_empty_db_returns_empty_vec() {
    let db = in_memory_db();
    let rows = db.export().unwrap();
    assert!(rows.is_empty());
}

// ── stats ─────────────────────────────────────────────────────────────────────

#[test]
fn stats_total_and_unique_counts() {
    let db = in_memory_db();

    db.insert(&make_cmd("git status", 0, "/home", "bash", "1"))
        .unwrap();
    db.insert(&make_cmd("git status", 0, "/home", "bash", "1"))
        .unwrap();
    db.insert(&make_cmd("cargo build", 0, "/home", "bash", "1"))
        .unwrap();

    let stats = db.stats(10).unwrap();
    assert_eq!(stats.total_commands, 3);
    assert_eq!(stats.unique_commands, 2);
}

#[test]
fn stats_error_rate_zero_when_all_succeed() {
    let db = in_memory_db();

    db.insert(&make_cmd("cmd-a", 0, "/home", "bash", "1"))
        .unwrap();
    db.insert(&make_cmd("cmd-b", 0, "/home", "bash", "1"))
        .unwrap();

    let stats = db.stats(10).unwrap();
    assert_eq!(stats.error_rate, 0.0);
}

#[test]
fn stats_error_rate_correct() {
    let db = in_memory_db();

    db.insert(&make_cmd("good", 0, "/home", "bash", "1"))
        .unwrap();
    db.insert(&make_cmd("bad", 1, "/home", "bash", "1"))
        .unwrap();

    let stats = db.stats(10).unwrap();
    assert_eq!(stats.error_rate, 50.0);
}

#[test]
fn stats_top_commands_ordered_by_count() {
    let db = in_memory_db();

    for _ in 0..5 {
        db.insert(&make_cmd("git status", 0, "/home", "bash", "1"))
            .unwrap();
    }
    for _ in 0..2 {
        db.insert(&make_cmd("cargo build", 0, "/home", "bash", "1"))
            .unwrap();
    }
    db.insert(&make_cmd("ls", 0, "/home", "bash", "1")).unwrap();

    let stats = db.stats(10).unwrap();
    assert_eq!(stats.top_commands[0].0, "git status");
    assert_eq!(stats.top_commands[0].1, 5);
    assert_eq!(stats.top_commands[1].0, "cargo build");
    assert_eq!(stats.top_commands[1].1, 2);
}

#[test]
fn stats_top_commands_respects_top_n() {
    let db = in_memory_db();

    for i in 0..10 {
        db.insert(&make_cmd(
            &format!("unique-cmd-{}", i),
            0,
            "/home",
            "bash",
            "1",
        ))
        .unwrap();
    }

    let stats = db.stats(3).unwrap();
    assert_eq!(stats.top_commands.len(), 3);
}

#[test]
fn stats_top_directories() {
    let db = in_memory_db();

    for _ in 0..4 {
        db.insert(&make_cmd("git status", 0, "/projects/recall", "bash", "1"))
            .unwrap();
    }
    db.insert(&make_cmd("ls", 0, "/home", "bash", "1")).unwrap();

    let stats = db.stats(10).unwrap();
    assert_eq!(stats.top_directories[0].0, "/projects/recall");
    assert_eq!(stats.top_directories[0].1, 4);
}

#[test]
fn stats_on_empty_db_returns_zeros() {
    let db = in_memory_db();
    let stats = db.stats(10).unwrap();

    assert_eq!(stats.total_commands, 0);
    assert_eq!(stats.unique_commands, 0);
    assert_eq!(stats.error_rate, 0.0);
    assert!(stats.top_commands.is_empty());
    assert!(stats.top_directories.is_empty());
    assert!(stats.most_active_hours.is_empty());
}
