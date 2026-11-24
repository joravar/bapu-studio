//! Local persistence: collections, environments, database connections, and history are stored
//! as plain JSON files under the OS app-data directory (via the `directories` crate), rather
//! than Electron's `localStorage`. This is a deliberate improvement, not just a port: plain
//! files are easy to back up, inspect, and (for collections) sync to a git-tracked folder --
//! continuing the "git-friendly collection storage" feature from the Electron app.
//!
//! Anything secret-bearing is never written into these JSON files. `save_databases` and
//! `save_environments` strip secret fields out to `crate::secrets` before serializing, and
//! `load_databases`/`load_environments` rehydrate them back in after reading. A file on disk
//! (or in a backup, or synced to cloud storage) therefore never contains a plaintext password,
//! only a `"db:<id>:password"`-style reference that is meaningless without this machine's OS
//! keychain.

use crate::secrets;
use crate::types::{Collection, DatabaseConnection, Environment, HistoryItem};
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

pub struct Storage {
    data_dir: PathBuf,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let dirs = directories::ProjectDirs::from("dev", "bapustudio", "Bapu Studio")
            .context("could not determine the OS application-data directory")?;
        let data_dir = dirs.data_dir().to_path_buf();
        fs::create_dir_all(&data_dir)
            .with_context(|| format!("creating app data directory at {}", data_dir.display()))?;
        Ok(Self { data_dir })
    }

    fn path(&self, file: &str) -> PathBuf {
        self.data_dir.join(file)
    }

    fn read_json<T: serde::de::DeserializeOwned + Default>(&self, file: &str) -> Result<T> {
        let path = self.path(file);
        if !path.exists() {
            return Ok(T::default());
        }
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
    }

    fn write_json<T: serde::Serialize>(&self, file: &str, value: &T) -> Result<()> {
        let path = self.path(file);
        let tmp_path = path.with_extension("json.tmp");
        let raw = serde_json::to_string_pretty(value)?;
        fs::write(&tmp_path, raw)
            .with_context(|| format!("writing {}", tmp_path.display()))?;
        fs::rename(&tmp_path, &path)
            .with_context(|| format!("finalizing {}", path.display()))?;
        Ok(())
    }

    // ---- Collections (no secrets live in a request's own fields today, so no stripping) ----

    pub fn load_collections(&self) -> Result<Vec<Collection>> {
        self.read_json("collections.json")
    }

    pub fn save_collections(&self, collections: &[Collection]) -> Result<()> {
        self.write_json("collections.json", &collections)
    }

    pub fn load_history(&self) -> Result<Vec<HistoryItem>> {
        self.read_json("history.json")
    }

    pub fn save_history(&self, history: &[HistoryItem]) -> Result<()> {
        self.write_json("history.json", &history)
    }

    // ---- Database connections (password + SSH secrets routed through the OS keychain) ----

    pub fn load_databases(&self) -> Result<Vec<DatabaseConnection>> {
        let mut dbs: Vec<DatabaseConnection> = self.read_json("databases.json")?;
        for db in &mut dbs {
            db.password = secrets::get_secret(&format!("db:{}:password", db.id)).ok();
            db.ssh_password = secrets::get_secret(&format!("db:{}:ssh_password", db.id)).ok();
            db.ssh_passphrase = secrets::get_secret(&format!("db:{}:ssh_passphrase", db.id)).ok();
            db.ssh_private_key = secrets::get_secret(&format!("db:{}:ssh_private_key", db.id)).ok();
        }
        Ok(dbs)
    }

    pub fn save_databases(&self, databases: &[DatabaseConnection]) -> Result<()> {
        let mut sanitized: Vec<DatabaseConnection> = Vec::with_capacity(databases.len());
        for db in databases {
            if let Some(pw) = &db.password {
                let _ = secrets::set_secret(&format!("db:{}:password", db.id), pw);
            }
            if let Some(pw) = &db.ssh_password {
                let _ = secrets::set_secret(&format!("db:{}:ssh_password", db.id), pw);
            }
            if let Some(p) = &db.ssh_passphrase {
                let _ = secrets::set_secret(&format!("db:{}:ssh_passphrase", db.id), p);
            }
            if let Some(k) = &db.ssh_private_key {
                let _ = secrets::set_secret(&format!("db:{}:ssh_private_key", db.id), k);
            }
            let mut clean = db.clone();
            clean.password = None;
            clean.ssh_password = None;
            clean.ssh_passphrase = None;
            clean.ssh_private_key = None;
            sanitized.push(clean);
        }
        self.write_json("databases.json", &sanitized)
    }

    pub fn delete_database_secrets(&self, id: &str) -> Result<()> {
        let _ = secrets::delete_secret(&format!("db:{}:password", id));
        let _ = secrets::delete_secret(&format!("db:{}:ssh_password", id));
        let _ = secrets::delete_secret(&format!("db:{}:ssh_passphrase", id));
        let _ = secrets::delete_secret(&format!("db:{}:ssh_private_key", id));
        Ok(())
    }

    // ---- Environments (only variables flagged isSecret are routed through the keychain) ----

    pub fn load_environments(&self) -> Result<Vec<Environment>> {
        let mut envs: Vec<Environment> = self.read_json("environments.json")?;
        for env in &mut envs {
            for var in &mut env.variables {
                if var.is_secret.unwrap_or(false) {
                    if let Ok(real) = secrets::get_secret(&format!("env:{}:{}", env.id, var.key)) {
                        var.value = real;
                    }
                }
            }
        }
        Ok(envs)
    }

    pub fn save_environments(&self, environments: &[Environment]) -> Result<()> {
        let mut sanitized: Vec<Environment> = Vec::with_capacity(environments.len());
        for env in environments {
            let mut clean = env.clone();
            for var in &mut clean.variables {
                if var.is_secret.unwrap_or(false) {
                    let _ = secrets::set_secret(&format!("env:{}:{}", env.id, var.key), &var.value);
                    var.value = String::new(); // never written to disk in plaintext
                }
            }
            sanitized.push(clean);
        }
        self.write_json("environments.json", &sanitized)
    }
}
