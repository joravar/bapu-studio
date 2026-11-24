//! OS-keychain-backed secret storage, replacing Electron's `safeStorage` IPC bridge.
//!
//! Unlike the Electron app (where `secureEncrypt`/`secureDecrypt` are exposed on
//! `window.bapuBridge` and reachable from any renderer-side JS, including a sandboxed script
//! that escapes its sandbox), this module is only callable from Rust code in this process —
//! there is no scripting engine binding that exposes it, so a pre-request/test script running
//! in `crate::script_engine` has no path to it at all.
//!
//! Secrets are stored in the OS credential store (Keychain on macOS, Credential Manager/DPAPI
//! on Windows, Secret Service/kwallet on Linux) via the `keyring` crate, keyed by a service
//! name + a logical key (e.g. "db:<connection-id>:password"). If no OS keychain backend is
//! available (some headless Linux setups), `keyring` returns an error, which we surface
//! honestly rather than silently falling back to plaintext.

use thiserror::Error;

const SERVICE_NAME: &str = "dev.bapustudio.app";

#[derive(Debug, Error)]
pub enum SecretsError {
    #[error("OS keychain is unavailable on this system: {0}")]
    Unavailable(String),
    #[error("no secret stored for this key")]
    NotFound,
}

/// Store a secret under `key` (e.g. "db:db-abc123:password", "env:env-default:API_TOKEN").
pub fn set_secret(key: &str, value: &str) -> Result<(), SecretsError> {
    let entry = keyring::Entry::new(SERVICE_NAME, key)
        .map_err(|e| SecretsError::Unavailable(e.to_string()))?;
    entry
        .set_password(value)
        .map_err(|e| SecretsError::Unavailable(e.to_string()))
}

/// Retrieve a previously stored secret. Returns `SecretsError::NotFound` if nothing is stored
/// under this key (not an error condition callers need to treat as a hard failure).
pub fn get_secret(key: &str) -> Result<String, SecretsError> {
    let entry = keyring::Entry::new(SERVICE_NAME, key)
        .map_err(|e| SecretsError::Unavailable(e.to_string()))?;
    match entry.get_password() {
        Ok(v) => Ok(v),
        Err(keyring::Error::NoEntry) => Err(SecretsError::NotFound),
        Err(e) => Err(SecretsError::Unavailable(e.to_string())),
    }
}

/// Delete a stored secret (e.g. when a database connection or env variable is removed).
/// Deleting a key that doesn't exist is not an error.
pub fn delete_secret(key: &str) -> Result<(), SecretsError> {
    let entry = keyring::Entry::new(SERVICE_NAME, key)
        .map_err(|e| SecretsError::Unavailable(e.to_string()))?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(SecretsError::Unavailable(e.to_string())),
    }
}

/// True if this platform's OS keychain backend is reachable right now. Call this once at
/// startup so the UI can show an honest warning banner instead of secrets silently failing
/// to save, mirroring the Electron app's `secure-store:is-encryption-available` check.
pub fn is_available() -> bool {
    // A real round-trip against a throwaway key is the only reliable way to check this —
    // keyring has no separate "is the backend up" probe.
    let probe_key = "__bapu_availability_probe__";
    if set_secret(probe_key, "probe").is_err() {
        return false;
    }
    let ok = get_secret(probe_key).is_ok();
    let _ = delete_secret(probe_key);
    ok
}
