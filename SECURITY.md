# Security Policy 🛡️

The Bapu Studio team takes the security of our software and user data extremely seriously. As a **local-first, privacy-focused developer cockpit**, we design our software to operate with **zero telemetry** and zero forced cloud connectivity.

---

## 📋 Supported Versions

We actively release security patches and updates for the following versions of Bapu Studio:

| Version | Supported | Notes |
| :--- | :---: | :--- |
| **v1.x** | ✅ **Supported** | Latest stable desktop releases (`.exe`, `.dmg`, Linux packages) |
| **< v1.0** | ❌ End of Support | Legacy pre-releases |

---

## 🔒 Security Architecture & Guarantees

Bapu Studio adheres to strict security defaults to protect developer secrets and database credentials:

* **100% Local-First Storage:** Database connection strings, API secrets, and `.env` variables are stored locally on your machine and are never transmitted to external third-party telemetry servers.
* **Encrypted at Rest:** Anything secret-bearing (database passwords, environment/secret-matrix variables, BYOK API keys) is encrypted before it touches disk, via Electron's OS-keychain-backed `safeStorage` (Keychain on macOS, DPAPI on Windows, Secret Service/kwallet on Linux). If no OS keychain backend is available, Bapu Studio stores the value as plaintext rather than silently pretending it's encrypted, and that fallback is visible, not hidden.
* **Electron Process Isolation:** The desktop app runs with strict Electron hardening defaults (`contextIsolation: true`, `nodeIntegration: false`, a minimal preload bridge exposing only named, single-purpose IPC methods — no generic `ipcRenderer.invoke` passthrough).
* **TLS Certificate Validation by Default:** Database connections over TLS (Postgres/MySQL/MongoDB/Redis, including managed providers like Neon, Supabase, Aiven, and AWS) validate the server's certificate by default. Skipping validation for a self-signed/dev certificate is an explicit, named opt-in on that connection, never a silent default.
* **Sandboxed Script Execution:** Pre-request and test scripts (the Postman-compatible `pm`/`bapu` scripting engine) run inside an isolated Web Worker with no access to `window`, `localStorage`, the database/IPC bridge, or any other app state beyond the specific request/response/environment data it's handed — and a timeout kills runaway/infinite-loop scripts. Importing a collection that contains such scripts shows an explicit warning before you run anything, since a script can still read and act on the current environment's variables by design (the same trade-off every Postman-style scripting engine makes) — only import collections from sources you trust.
* **Secret Masking:** Environment variables marked as secrets are masked in the UI to prevent accidental exposure during screensharing or live streams.

---

## 🚨 Reporting a Vulnerability

If you discover a security vulnerability in Bapu Studio, please report it to us **privately**. Please do **NOT** create a public GitHub issue for security vulnerabilities.

### Preferred Reporting Channels
* **Email:** Send details to **support@bapustudio.dev**
* **GitHub Private Vulnerability Reporting:** Submit a advisory directly via [GitHub Security Advisories](https://github.com/joravar/bapu-studio/security/advisories/new)

### What to Include in Your Report
To help us triage and respond to your report quickly, please include:
1. **Description of the Vulnerability:** Potential impact and affected components (e.g., Database Studio IPC driver, API client parser, Secret Matrix storage).
2. **Proof of Concept (PoC) / Reproduction Steps:** Clear step-by-step instructions to reproduce the vulnerability.
3. **Environment:** OS version (Windows, macOS, Linux) and Bapu Studio release version.

---

## ⏱️ Response & Disclosure SLA

We are committed to coordinated, responsible disclosure:

* **Initial Acknowledgment:** Within **48 hours** of receiving your report.
* **Vulnerability Assessment & Triage:** Within **5 business days**.
* **Security Patch Release:** Critical vulnerabilities will be patched and published via a new GitHub Release within **14 days**.

Once a security fix has been released, we will publish a GitHub Security Advisory crediting your discovery (unless you prefer to remain anonymous).
