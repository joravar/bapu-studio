# Security Policy 🛡️

The Bapu Studio team takes the security of our software and user data seriously. As a **local-first, privacy-focused developer cockpit**, Bapu Studio operates with **zero telemetry** and zero forced cloud connectivity.

---

## 📋 Supported Versions

We actively release security patches and updates for the following versions of Bapu Studio:

| Version | Supported | Notes |
| :--- | :---: | :--- |
| **v2.x (Native Rust)** | ✅ **Supported** | Pure-Rust native desktop releases (`egui` / `eframe` — `.exe`, macOS `.tar.gz`, Linux `.tar.gz`) |
| **v1.4.x (Legacy Electron)** | ⚠️ **Security-only** | Maintenance branch (`v1.*` tags) |
| **< v1.4** | ❌ End of Support | Legacy pre-releases |

---

## 🔒 Security Architecture & Guarantees (v2.0 Native Rust)

Bapu Studio adheres to strict security defaults to protect developer secrets and database credentials:

* **Zero Browser / Node / JVM Attack Surface:** Bapu Studio v2.0 is compiled directly to native machine code in memory-safe Rust (`egui` / `eframe`). There is no Chromium renderer, no Node.js runtime, no IPC bridge, and no bundled JVM.
* **100% Local-First Storage:** Collections, SQL tabs/snippets, environments, and history are stored locally in your OS application-data directory (`%APPDATA%\bapustudio\Bapu Studio\data` on Windows, `~/Library/Application Support/dev.bapustudio.Bapu-Studio` on macOS, `~/.local/share/bapustudio` on Linux) and are never transmitted to any telemetry server.
* **OS Keychain Secret Protection (`keyring`):** Database passwords, SSH passwords/private keys/passphrases, secret-flagged environment & global variables, and BYOK AI Copilot API keys are routed directly to your OS credential vault (**Windows Credential Manager / DPAPI**, **macOS Keychain**, **Linux Secret Service / KWallet**) and stripped before JSON metadata touches disk. Deleting a database connection, environment, or secret variable removes its corresponding keychain entry.
* **Air-Gapped Pure-Rust JS Script Sandbox (`boa_engine`):** Pre-request and test assertion scripts (`bapu.*` / `pm.*`) execute inside a dedicated `boa_engine` ECMAScript interpreter with **zero** filesystem, network (`fetch`), OS process, or keychain bindings, enforced with a strict 5-second execution timeout on a worker thread.
* **TLS Certificate Validation & Mutual TLS (mTLS):** HTTP requests and TLS-enabled database connections validate server certificates by default (`rustls`), with first-class support for custom Root CA PEMs, client certificates/keys (mTLS), and pure-Rust `russh` SSH bastion port forwarding.
* **Parameterized Transactional Grid Edits:** Staged Data Grid edits, inserts, and deletes bind values securely or quote dialect-specific literals inside real transactions (`BEGIN` / `COMMIT` / `ROLLBACK` where supported by the target engine).
* **Secret Masking:** Password fields, secret environment variables, and API keys are masked in the UI to prevent accidental exposure during screensharing.

---

## 🚨 Reporting a Vulnerability

If you discover a security vulnerability in Bapu Studio, please report it **privately** — do **NOT** open a public GitHub issue.

### Preferred Reporting Channels
* **Email:** **support@bapustudio.dev**
* **GitHub Private Vulnerability Reporting:** Submit an advisory directly via [GitHub Security Advisories](https://github.com/joravar/bapu-studio/security/advisories/new)

### What to Include in Your Report
1. **Description of the Vulnerability:** Potential impact and affected crate/module (e.g., `bapu-core::storage`, `bapu-core::script_engine`, specific DB driver).
2. **Proof of Concept (PoC) / Reproduction Steps:** Clear step-by-step instructions to reproduce.
3. **Environment:** OS version (Windows, macOS, Linux) and Bapu Studio version.

---

## ⏱️ Response & Disclosure SLA

* **Initial Acknowledgment:** Within **48 hours** of receiving your report.
* **Vulnerability Assessment & Triage:** Within **5 business days**.
* **Security Patch Release:** Critical vulnerabilities patched and published via a new GitHub Release within **14 days**.
