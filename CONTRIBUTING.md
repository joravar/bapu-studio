# Contributing to Bapu Studio ⚡

First off, thank you for considering contributing to **Bapu Studio**! 

Bapu Studio is an open-source, local-first developer cockpit combining an **API Client**, **Database Studio**, and **Secrets Matrix**. Our goal is to provide a fast, zero-telemetry, offline-first alternative to bloated SaaS developer tools.

---

## 📜 Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md). Please read it before contributing.

---

## 🛠️ How Can I Contribute?

### 1. Reporting Bugs
Before opening a new issue, please search existing [GitHub Issues](https://github.com/joravar/bapu-studio/issues) to avoid duplicates.

When filing a bug report, please include:
* **Operating System:** (Windows 11, macOS Sequoia, Ubuntu 24.04, etc.)
* **App Version:** (e.g., v1.0.0)
* **Steps to Reproduce:** Clear step-by-step instructions.
* **Expected vs. Actual Behavior:** What did you expect to happen vs. what actually happened?
* **Logs:** Any error output from the terminal (run the binary with `RUST_LOG=debug` for verbose logs).

> 🛡️ **Security Vulnerabilities:** Please do **NOT** open public issues for security vulnerabilities. Email details directly to **support@bapustudio.dev**.

### 2. Suggesting Features
We welcome feature proposals! When opening a feature request:
* Explain **why** this feature is useful to developers.
* Describe the **user workflow** (how it fits into API Studio, Database Studio, or Secrets Matrix).
* Keep in mind Bapu Studio's core philosophy: **simplicity, speed, local privacy, and zero telemetry**.

---

## 💻 Development Setup

### Prerequisites
* **Rust** (stable toolchain, via [rustup](https://rustup.rs))
* **Git**
* On Linux, the GUI toolkit's dev headers: `pkg-config cmake libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev libgtk-3-dev libdbus-1-dev` (see `.github/workflows/native-ci.yml` for the exact `apt-get` line). Windows and macOS need nothing extra beyond the Rust toolchain.

### Local Environment Setup

1. **Fork and Clone the Repository:**
   ```bash
   git clone https://github.com/YOUR-USERNAME/bapu-studio.git
   cd bapu-studio/native
   ```

2. **Run the Desktop Cockpit:**
   ```bash
   cargo run -p bapu-app
   ```

3. **Run the Test Suite:**
   ```bash
   cargo test --workspace
   ```

4. **Format & Lint Before Committing:**
   ```bash
   cargo fmt --all
   cargo clippy --workspace -- -D warnings
   ```

---

## 🔀 Pull Request (PR) Workflow

1. **Create a Feature Branch:**
   ```bash
   git checkout -b feat/your-feature-name
   # or
   git checkout -b fix/your-bug-fix
   ```

2. **Code Guidelines & Architecture Principles:**
   * **Rust:** Run `cargo fmt` and `cargo clippy --workspace` clean before opening a PR. Prefer propagating errors with `anyhow`/`thiserror` over `unwrap()`/`expect()` outside of tests.
   * **UI:** The `egui`/`eframe` immediate-mode interface lives in `native/crates/bapu-app/src/ui/`; shared logic (HTTP, DB drivers, secrets, scripting) lives in `native/crates/bapu-core/`.
   * **Zero Telemetry:** Never add network tracking, analytics, or phone-home telemetry calls.
   * **Pure-Rust Drivers:** Database connectors must be pure-Rust crates that run locally, without bundling Node, the JVM, or other heavy external runtimes.

3. **Commit Message Format:**
   We follow conventional commit formatting:
   * `feat: add GraphQL query variable autocomplete`
   * `fix: prevent crash on corrupted SQLite file load`
   * `docs: update troubleshooting guide`
   * `chore: update dependencies`

4. **Submit Pull Request:**
   * Push your branch to your fork: `git push origin feat/your-feature-name`
   * Open a PR against the `main` branch of `joravar/bapu-studio`.
   * Fill out the PR template explaining your changes and linking relevant issues.

---

## 📄 License & Contributor Agreement

Bapu Studio is licensed under the **GNU Affero General Public License v3.0 (AGPLv3)**.

By submitting a Pull Request to this project, you certify under the **Developer Certificate of Origin (DCO)** that your contribution is your original work and that you have the right to submit it under the project's license terms.
