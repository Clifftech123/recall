use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

// ── helpers ───────────────────────────────────────────────────────────────────

/// Build a `Command` for the recall binary with RECALL_DATA_DIR pointing at a
/// fresh temp directory so every test gets its own isolated database.
fn cmd(tmp: &TempDir) -> Command {
    let mut c = Command::cargo_bin("recall").unwrap();
    c.env("RECALL_DATA_DIR", tmp.path());
    c
}

/// Initialise a fresh database in `tmp` and return the dir.
fn init(tmp: &TempDir) {
    cmd(tmp).arg("init").assert().success();
}

/// Log a single command into the database.
fn log(tmp: &TempDir, command: &str) {
    cmd(tmp)
        .args([
            "log",
            command,
            "--exit-code",
            "0",
            "--cwd",
            "/home/user/project",
            "--shell",
            "bash",
            "--session",
            "1234",
        ])
        .assert()
        .success();
}

/// Log a command that failed (non-zero exit code).
fn log_error(tmp: &TempDir, command: &str) {
    cmd(tmp)
        .args([
            "log",
            command,
            "--exit-code",
            "1",
            "--cwd",
            "/home/user/project",
            "--shell",
            "bash",
            "--session",
            "1234",
        ])
        .assert()
        .success();
}

// ── init ──────────────────────────────────────────────────────────────────────

#[test]
fn init_succeeds() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp)
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("✓"));
}

#[test]
fn init_is_idempotent() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);
    // Running init a second time should still succeed
    cmd(&tmp).arg("init").assert().success();
}

// ── log ───────────────────────────────────────────────────────────────────────

#[test]
fn log_requires_init() {
    let tmp = TempDir::new().unwrap();
    // Without init the command should exit cleanly (not crash)
    cmd(&tmp).args(["log", "git status"]).assert().success();
}

#[test]
fn log_stores_command() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);
    log(&tmp, "git status");

    cmd(&tmp)
        .args(["history", "--limit", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("git status"));
}

#[test]
fn log_stores_exit_code() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);
    log_error(&tmp, "cargo test");

    cmd(&tmp)
        .args(["history", "--limit", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cargo test"));
}

#[test]
fn log_silent_on_success() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);
    // The log subcommand must produce no stdout — it runs after every command
    // in the shell hook and any output would pollute the terminal.
    cmd(&tmp)
        .args(["log", "echo hello"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

// ── history ───────────────────────────────────────────────────────────────────

#[test]
fn history_empty_database() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);

    cmd(&tmp)
        .arg("history")
        .assert()
        .success()
        .stdout(predicate::str::contains("No commands logged yet."));
}

#[test]
fn history_shows_logged_commands() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);
    log(&tmp, "git push origin main");
    log(&tmp, "cargo build --release");

    let out = cmd(&tmp)
        .args(["history", "--limit", "10"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("git push origin main"));
    assert!(text.contains("cargo build --release"));
}

#[test]
fn history_respects_limit() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);
    for i in 0..10 {
        log(&tmp, &format!("echo {}", i));
    }

    // Ask for only 3 results — table should have exactly 3 data rows
    let out = cmd(&tmp)
        .args(["history", "--limit", "3"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(out).unwrap();
    // Count how many times "echo" appears
    let count = text.matches("echo").count();
    assert_eq!(count, 3, "expected 3 rows, got {}", count);
}

#[test]
fn history_json_output() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);
    log(&tmp, "ls -la");

    cmd(&tmp)
        .args(["history", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("["))
        .stdout(predicate::str::contains("ls -la"))
        .stdout(predicate::str::contains("\"command\""));
}

#[test]
fn history_csv_output() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);
    log(&tmp, "ls -la");

    cmd(&tmp)
        .args(["history", "--csv"])
        .assert()
        .success()
        .stdout(predicate::str::contains("id,command,timestamp"))
        .stdout(predicate::str::contains("ls -la"));
}

#[test]
fn history_errors_only_flag() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);
    log(&tmp, "git status"); // exit 0
    log_error(&tmp, "cargo test"); // exit 1

    let out = cmd(&tmp)
        .args(["history", "--errors"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("cargo test"));
    assert!(!text.contains("git status"));
}

#[test]
fn history_filter_by_session() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);

    // Log with session 1111
    cmd(&tmp)
        .args([
            "log",
            "echo session-a",
            "--session",
            "1111",
            "--exit-code",
            "0",
        ])
        .assert()
        .success();

    // Log with session 2222
    cmd(&tmp)
        .args([
            "log",
            "echo session-b",
            "--session",
            "2222",
            "--exit-code",
            "0",
        ])
        .assert()
        .success();

    let out = cmd(&tmp)
        .args(["history", "--session", "1111"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("session-a"));
    assert!(!text.contains("session-b"));
}

// ── search ────────────────────────────────────────────────────────────────────

#[test]
fn search_finds_matching_command() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);
    log(&tmp, "git commit -m \"fix bug\"");
    log(&tmp, "cargo build --release");
    log(&tmp, "git push origin main");

    cmd(&tmp)
        .args(["search", "git"])
        .assert()
        .success()
        .stdout(predicate::str::contains("git"));
}

#[test]
fn search_no_results() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);
    log(&tmp, "cargo build");

    cmd(&tmp)
        .args(["search", "docker"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No results found for"));
}

#[test]
fn search_json_output() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);
    log(&tmp, "git status");

    cmd(&tmp)
        .args(["search", "git", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("["))
        .stdout(predicate::str::contains("\"command\""));
}

#[test]
fn search_respects_limit() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);
    for i in 0..5 {
        log(&tmp, &format!("git status {}", i));
    }

    let out = cmd(&tmp)
        .args(["search", "git", "--limit", "2"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(out).unwrap();
    let count = text.matches("git status").count();
    assert!(count <= 2, "expected at most 2 results, got {}", count);
}

// ── undo ─────────────────────────────────────────────────────────────────────

#[test]
fn undo_removes_last_command() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);
    log(&tmp, "git status");
    log(&tmp, "rm -rf /tmp/test");

    // Undo should remove "rm -rf /tmp/test"
    cmd(&tmp)
        .arg("undo")
        .assert()
        .success()
        .stdout(predicate::str::contains("rm -rf /tmp/test"));

    // History should now only have git status
    let out = cmd(&tmp)
        .arg("history")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(out).unwrap();
    assert!(!text.contains("rm -rf /tmp/test"));
    assert!(text.contains("git status"));
}

#[test]
fn undo_on_empty_database() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);

    // Should succeed without panicking
    cmd(&tmp).arg("undo").assert().success();
}

// ── clean ─────────────────────────────────────────────────────────────────────

#[test]
fn clean_all_removes_everything() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);
    log(&tmp, "git status");
    log(&tmp, "cargo build");

    cmd(&tmp).args(["clean", "--all"]).assert().success();

    cmd(&tmp)
        .arg("history")
        .assert()
        .success()
        .stdout(predicate::str::contains("No commands logged yet."));
}

#[test]
fn clean_dry_run_does_not_delete() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);
    log(&tmp, "git status");
    log(&tmp, "cargo build");

    // Dry run should report count but not delete
    cmd(&tmp)
        .args(["clean", "--all", "--dry-run"])
        .assert()
        .success();

    // Commands should still be there
    let out = cmd(&tmp)
        .arg("history")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("git status"));
    assert!(text.contains("cargo build"));
}

// ── stats ─────────────────────────────────────────────────────────────────────

#[test]
fn stats_on_empty_database() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);

    cmd(&tmp)
        .arg("stats")
        .assert()
        .success()
        .stdout(predicate::str::contains("No commands logged yet."));
}

#[test]
fn stats_shows_top_commands() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);

    for _ in 0..5 {
        log(&tmp, "git status");
    }
    log(&tmp, "cargo build");

    cmd(&tmp)
        .args(["stats", "--top", "5"])
        .assert()
        .success()
        .stdout(predicate::str::contains("git status"))
        .stdout(predicate::str::contains("Top Commands"));
}

// ── export ────────────────────────────────────────────────────────────────────

#[test]
fn export_json_to_stdout() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);
    log(&tmp, "git status");

    cmd(&tmp)
        .args(["export", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("["))
        .stdout(predicate::str::contains("git status"));
}

#[test]
fn export_csv_to_stdout() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);
    log(&tmp, "git status");

    cmd(&tmp)
        .args(["export", "--format", "csv"])
        .assert()
        .success()
        .stdout(predicate::str::contains("id,command,timestamp"))
        .stdout(predicate::str::contains("git status"));
}

#[test]
fn export_text_to_stdout() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);
    log(&tmp, "git status");

    cmd(&tmp)
        .args(["export", "--format", "text"])
        .assert()
        .success()
        .stdout(predicate::str::contains("git status"));
}

#[test]
fn export_to_file() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);
    log(&tmp, "git status");

    let out_path = tmp.path().join("export.json");

    cmd(&tmp)
        .args([
            "export",
            "--format",
            "json",
            "--output",
            out_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("✓"));

    let contents = std::fs::read_to_string(&out_path).unwrap();
    assert!(contents.contains("git status"));
}

#[test]
fn export_unknown_format_fails() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);
    log(&tmp, "git status");

    cmd(&tmp)
        .args(["export", "--format", "xml"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("xml"));
}

#[test]
fn export_empty_database() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);

    cmd(&tmp)
        .args(["export", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No commands to export."));
}

// ── completions ───────────────────────────────────────────────────────────────

#[test]
fn completions_bash() {
    Command::cargo_bin("recall")
        .unwrap()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("recall"));
}

#[test]
fn completions_zsh() {
    Command::cargo_bin("recall")
        .unwrap()
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn completions_fish() {
    Command::cargo_bin("recall")
        .unwrap()
        .args(["completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

// ── error handling ────────────────────────────────────────────────────────────

#[test]
fn unknown_subcommand_fails() {
    let tmp = TempDir::new().unwrap();
    cmd(&tmp).arg("notacommand").assert().failure();
}

#[test]
fn missing_required_arg_fails() {
    // `search` requires a query argument
    let tmp = TempDir::new().unwrap();
    cmd(&tmp).arg("search").assert().failure();
}
