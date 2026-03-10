use chrono::Utc;
use serde::{Deserialize, Serialize};

/// A single logged command entry stored in the database.
#[derive(Serialize, Deserialize)]
pub struct Command {
    pub id: i64,
    pub command: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    pub exit_code: i32,
    pub shell: Option<String>,
    pub hostname: Option<String>,
    pub metadata: Option<String>,
}

pub struct Stats {
    pub total_commands: u64,
    pub unique_commands: u64,
    pub date_range: (String, String),     // earliest, latest
    pub top_commands: Vec<(String, u64)>, // command → count

    pub most_active_hours: Vec<(u8, u64)>,   // hour → count
    pub error_rate: f64,                     // percentage of non-zero exit codes
    pub top_directories: Vec<(String, u64)>, // cwd → count
}

impl Command {
    /// Returns a human-readable relative time string such as "3 minutes ago".
    pub fn relative_time(&self) -> String {
        let now = Utc::now();
        let duration = now.signed_duration_since(self.timestamp);

        let secs = duration.num_seconds();
        if secs < 60 {
            return format!("{} second{} ago", secs, if secs == 1 { "" } else { "s" });
        }

        let mins = duration.num_minutes();
        if mins < 60 {
            return format!("{} minute{} ago", mins, if mins == 1 { "" } else { "s" });
        }

        let hours = duration.num_hours();
        if hours < 24 {
            return format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" });
        }

        let days = duration.num_days();
        format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
    }

    /// Returns the display-friendly working directory, replacing the home prefix with `~`.
    pub fn display_cwd(&self) -> String {
        match &self.cwd {
            None => String::from("—"),
            Some(cwd) => {
                if let Some(home) = dirs::home_dir() {
                    if let Some(stripped) = cwd.strip_prefix(home.to_str().unwrap_or("")) {
                        return format!("~{}", stripped);
                    }
                }
                cwd.clone()
            }
        }
    }
}
