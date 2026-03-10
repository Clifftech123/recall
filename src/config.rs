use crate::error::RecallErrors;
use std::path::PathBuf;

/// Holds the resolved paths that Recall uses at runtime.
///
/// All paths are derived from the user's home directory under `~/.recall/`,
/// or from the `RECALL_DATA_DIR` environment variable if set — useful for
/// testing or running multiple isolated instances.
#[derive(Debug)]
pub struct Config {
    /// The directory where all Recall data lives. Defaults to `~/.recall/`.
    pub data_dir: PathBuf,

    /// The full path to the SQLite database file (`~/.recall/history.db`).
    pub db_path: PathBuf,
}

impl Config {
    /// Resolves all paths and returns a `Config` instance.
    ///
    /// Resolution order for the data directory:
    /// 1. `RECALL_DATA_DIR` environment variable if set
    /// 2. `~/.recall/` derived from the system home directory
    ///
    /// Returns `RecallErrors::NoHomeDir` if the home directory cannot be
    /// determined and `RECALL_DATA_DIR` is not set.
    pub fn load() -> Result<Self, RecallErrors> {
        let data_dir = if let Some(override_dir) = std::env::var_os("RECALL_DATA_DIR") {
            PathBuf::from(override_dir)
        } else {
            let home = dirs::home_dir().ok_or(RecallErrors::NoHomeDir)?;
            home.join(".recall")
        };

        let db_path = data_dir.join("history.db");

        Ok(Self { data_dir, db_path })
    }

    /// Creates the data directory if it does not already exist.
    ///
    /// This is called by `recall init` before the database is opened.
    /// It is safe to call multiple times — it will not fail if the
    /// directory already exists.
    pub fn ensure_dir(&self) -> Result<(), RecallErrors> {
        std::fs::create_dir_all(&self.data_dir).map_err(RecallErrors::Io)?;
        Ok(())
    }

    /// Returns `true` if the data directory and database file both exist on disk.
    ///
    /// This is a read-only check — it does not create anything. Use
    /// `ensure_dir()` followed by `db::init()` if you need to set up the
    /// environment.
    pub fn is_initialized(&self) -> bool {
        self.data_dir.exists() && self.db_path.exists()
    }
}
