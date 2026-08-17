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
* **Console Logs:** Any error output from Developer Tools (`Ctrl+Shift+I` or `Cmd+Option+I`).

> 🛡️ **Security Vulnerabilities:** Please do **NOT** open public issues for security vulnerabilities. Email details directly to **support@bapustudio.dev**.

### 2. Suggesting Features
We welcome feature proposals! When opening a feature request:
* Explain **why** this feature is useful to developers.
* Describe the **user workflow** (how it fits into API Studio, Database Studio, or Secrets Matrix).
* Keep in mind Bapu Studio's core philosophy: **simplicity, speed, local privacy, and zero telemetry**.

---

## 💻 Development Setup

### Prerequisites
* **Node.js** v18+ 
* **npm** v9+
* **Git**

### Local Environment Setup

1. **Fork and Clone the Repository:**
   ```bash
   git clone https://github.com/YOUR-USERNAME/bapu-studio.git
   cd bapu-studio
   ```

2. **Install Dependencies:**
   ```bash
   npm install
   ```

3. **Run Development Cockpit:**
   ```bash
   # Run Vite frontend in browser mode
   npm run dev

   # Run full desktop application in Electron
   npm run desktop
   ```

4. **Run Test Suite:**
   ```bash
   npm run test
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
   * **TypeScript:** Strictly typed code. Avoid `any` where possible.
   * **Styling:** Use tailored HSL dark theme Vanilla CSS tokens in `src/index.css`. Avoid adding heavy third-party CSS utility frameworks.
   * **Zero Telemetry:** Never add network tracking, analytics, or phone-home telemetry calls.
   * **Pure JS Database Drivers:** Database connectors must run locally without heavy external background daemons.

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
