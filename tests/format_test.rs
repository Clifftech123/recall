use chrono::TimeZone;
use recall::models::Command;

fn make_command(id: i64, command: &str, exit_code: i32) -> Command {
    Command {
        id,
        command: command.to_string(),
        timestamp: chrono::Utc.with_ymd_and_hms(2026, 1, 15, 10, 0, 0).unwrap(),
        session_id: Some("sess1".to_string()),
        cwd: Some("/home/cliff".to_string()),
        exit_code,
        shell: Some("bash".to_string()),
        hostname: Some("cliffbox".to_string()),
        metadata: None,
    }
}

// ── JSON ──────────────────────────────────────────────────────────────────────

#[test]
fn json_output_contains_command_text() {
    let cmds = vec![make_command(1, "git status", 0)];
    let json = serde_json::to_string_pretty(&cmds).unwrap();
    assert!(json.contains("git status"));
}

#[test]
fn json_output_contains_all_fields() {
    let cmds = vec![make_command(1, "cargo build", 0)];
    let json = serde_json::to_string_pretty(&cmds).unwrap();
    assert!(json.contains("\"id\""));
    assert!(json.contains("\"command\""));
    assert!(json.contains("\"timestamp\""));
    assert!(json.contains("\"session_id\""));
    assert!(json.contains("\"cwd\""));
    assert!(json.contains("\"exit_code\""));
    assert!(json.contains("\"shell\""));
    assert!(json.contains("\"hostname\""));
}

#[test]
fn json_output_is_array() {
    let cmds = vec![make_command(1, "ls", 0), make_command(2, "pwd", 0)];
    let json = serde_json::to_string_pretty(&cmds).unwrap();
    assert!(json.trim_start().starts_with('['));
    assert!(json.trim_end().ends_with(']'));
}

#[test]
fn json_empty_slice_is_empty_array() {
    let cmds: Vec<Command> = vec![];
    let json = serde_json::to_string_pretty(&cmds).unwrap();
    assert_eq!(json.trim(), "[]");
}

#[test]
fn json_exit_code_preserved() {
    let cmds = vec![make_command(1, "false", 1)];
    let json = serde_json::to_string_pretty(&cmds).unwrap();
    assert!(json.contains("\"exit_code\": 1"));
}

#[test]
fn json_null_fields_serialised() {
    let cmd = Command {
        id: 1,
        command: "echo hi".to_string(),
        timestamp: chrono::Utc.with_ymd_and_hms(2026, 1, 15, 10, 0, 0).unwrap(),
        session_id: None,
        cwd: None,
        exit_code: 0,
        shell: None,
        hostname: None,
        metadata: None,
    };
    let json = serde_json::to_string_pretty(&[cmd]).unwrap();
    assert!(json.contains("\"session_id\": null"));
    assert!(json.contains("\"cwd\": null"));
    assert!(json.contains("\"shell\": null"));
    assert!(json.contains("\"hostname\": null"));
}

#[test]
fn json_multiple_commands_all_present() {
    let cmds = vec![
        make_command(1, "git pull", 0),
        make_command(2, "git push", 0),
        make_command(3, "git log", 0),
    ];
    let json = serde_json::to_string_pretty(&cmds).unwrap();
    assert!(json.contains("git pull"));
    assert!(json.contains("git push"));
    assert!(json.contains("git log"));
}

// ── CSV ───────────────────────────────────────────────────────────────────────

fn commands_to_csv(cmds: &[Command]) -> String {
    let mut buf = Vec::new();
    {
        let mut writer = csv::Writer::from_writer(&mut buf);
        writer
            .write_record(&[
                "id",
                "command",
                "timestamp",
                "session_id",
                "cwd",
                "exit_code",
                "shell",
                "hostname",
            ])
            .unwrap();
        for cmd in cmds {
            writer
                .write_record(&[
                    cmd.id.to_string(),
                    cmd.command.clone(),
                    cmd.timestamp.to_rfc3339(),
                    cmd.session_id.clone().unwrap_or_default(),
                    cmd.cwd.clone().unwrap_or_default(),
                    cmd.exit_code.to_string(),
                    cmd.shell.clone().unwrap_or_default(),
                    cmd.hostname.clone().unwrap_or_default(),
                ])
                .unwrap();
        }
        writer.flush().unwrap();
    }
    String::from_utf8(buf).unwrap()
}

#[test]
fn csv_has_header_row() {
    let cmds = vec![make_command(1, "ls", 0)];
    let csv = commands_to_csv(&cmds);
    let first_line = csv.lines().next().unwrap();
    assert_eq!(
        first_line,
        "id,command,timestamp,session_id,cwd,exit_code,shell,hostname"
    );
}

#[test]
fn csv_row_count_matches_commands() {
    let cmds = vec![
        make_command(1, "ls", 0),
        make_command(2, "pwd", 0),
        make_command(3, "whoami", 0),
    ];
    let csv = commands_to_csv(&cmds);
    // header + 3 data rows
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 4);
}

#[test]
fn csv_contains_command_text() {
    let cmds = vec![make_command(1, "cargo test", 0)];
    let csv = commands_to_csv(&cmds);
    assert!(csv.contains("cargo test"));
}

#[test]
fn csv_exit_code_in_row() {
    let cmds = vec![make_command(1, "bad_cmd", 127)];
    let csv = commands_to_csv(&cmds);
    assert!(csv.contains("127"));
}

#[test]
fn csv_empty_slice_only_has_header() {
    let cmds: Vec<Command> = vec![];
    let csv = commands_to_csv(&cmds);
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].starts_with("id,"));
}

#[test]
fn csv_optional_fields_empty_when_none() {
    let cmd = Command {
        id: 1,
        command: "echo hi".to_string(),
        timestamp: chrono::Utc.with_ymd_and_hms(2026, 1, 15, 10, 0, 0).unwrap(),
        session_id: None,
        cwd: None,
        exit_code: 0,
        shell: None,
        hostname: None,
        metadata: None,
    };
    let csv = commands_to_csv(&[cmd]);
    // All optional fields should be empty strings, not "null"
    assert!(!csv.contains("null"));
}

#[test]
fn csv_id_is_correct() {
    let cmds = vec![make_command(42, "ls", 0)];
    let csv = commands_to_csv(&cmds);
    let data_line = csv.lines().nth(1).unwrap();
    assert!(data_line.starts_with("42,"));
}

#[test]
fn csv_commands_with_commas_are_quoted() {
    let cmd = Command {
        id: 1,
        command: "echo hello, world".to_string(),
        timestamp: chrono::Utc.with_ymd_and_hms(2026, 1, 15, 10, 0, 0).unwrap(),
        session_id: None,
        cwd: None,
        exit_code: 0,
        shell: None,
        hostname: None,
        metadata: None,
    };
    let csv = commands_to_csv(&[cmd]);
    // The csv crate should quote the field containing a comma
    assert!(csv.contains("\"echo hello, world\""));
}
