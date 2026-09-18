# ⚡ Bapu Studio — The Honest Open-Source Developer Cockpit

<div align="center">

[![License: AGPL v3](https://img.shields.io/badge/License-AGPLv3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Bapu Studio CI](https://github.com/joravar/bapu-studio/actions/workflows/ci.yml/badge.svg)](https://github.com/joravar/bapu-studio/actions/workflows/ci.yml)
[![Release: v1.1.0](https://img.shields.io/badge/Release-v1.1.0-emerald.svg)](https://github.com/joravar/bapu-studio/releases/tag/v1.1.0)
[![Platforms](https://img.shields.io/badge/Platforms-Windows%20%7C%20macOS%20%7C%20Linux-informational.svg)](https://github.com/joravar/bapu-studio/releases/tag/v1.1.0)
[![Website](https://img.shields.io/badge/Website-Live-brightgreen.svg)](https://joravar.github.io/bapu-studio/)
[![Sponsor](https://img.shields.io/badge/GitHub-Sponsor-pink.svg)](https://github.com/sponsors/joravar)

**The honest, lightweight, local-first developer cockpit combining an API Client, Multi-Tab Database Studio, Real-Time Streams, and Secrets Matrix into one seamless desktop application.**

[📦 Downloads](#-downloads) • [✨ New in v1.1.0](#-whats-new-in-v110) • [🌐 Live Website](https://joravar.github.io/bapu-studio/) • [📊 Comparison](#-feature-comparison) • [🏗️ Architecture](#️-architecture--technology-stack) • [🤖 AI Copilot](#-ai-copilot--current-status) • [💖 Sponsor](#-support--github-sponsors) • [📄 License](#-license)

</div>

---

## 🌟 Why Bapu Studio?

Developers are tired of running several bloated, heavy tools simultaneously just to test an endpoint, run a database query, and check a `.env` secret.

**Bapu Studio** stands for **simplicity, truth, zero bloat, and complete local privacy**.

```
┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│                                           BAPU STUDIO                                           │
├──────────────────────────────┬──────────────────────────────┬───────────────────────────────────┤
│        🚀 API CLIENT         │      🗄️ DATABASE STUDIO      │        🔐 SECRETS & STREAMS       │
│  • REST, GraphQL & cURL      │  • Multi-Tab SQL Scratchpads │  • Scoped .env Matrix             │
│  • Pre/Post Script Engine    │  • Postgres, MySQL, Mongo    │  • Variable Interpolation         │
│  • Postman v2.1 & OpenAPI    │  • ⚡ Query EXPLAIN Plans    │  • WebSocket & SSE Stream Studio  │
│  • Sub-ms Latency Benchmarks │  • Dual Grid / Console Mode  │  • Custom Root CA & mTLS Support  │
└──────────────────────────────┴──────────────────────────────┴───────────────────────────────────┘
```

---

## 📦 Downloads

Direct native installer packages are available on [GitHub Releases (v1.1.0)](https://github.com/joravar/bapu-studio/releases/tag/v1.1.0):

| Operating System | Installer Package | Format | Status |
| :--- | :--- | :--- | :--- |
| **🍏 macOS (Apple Silicon)** | [**Download macOS DMG**](https://github.com/joravar/bapu-studio/releases/download/v1.1.0/Bapu-Studio-1.1.0-mac-arm64.dmg) <br> [Download macOS Zip](https://github.com/joravar/bapu-studio/releases/download/v1.1.0/Bapu-Studio-1.1.0-mac-arm64.zip) | `.dmg` & `.zip` | ✅ Signed & Notarized by Apple |
| **🪟 Windows** | [**Download Windows Installer**](https://github.com/joravar/bapu-studio/releases/download/v1.1.0/Bapu-Studio-Setup-1.1.0.exe) <br> [Download Portable .exe](https://github.com/joravar/bapu-studio/releases/download/v1.1.0/Bapu-Studio-Portable-1.1.0.exe) <br> [Download Windows Zip](https://github.com/joravar/bapu-studio/releases/download/v1.1.0/Bapu-Studio-1.1.0-win-x64.zip) | `.exe` (NSIS & Portable) | ✅ Ready to install / run |
| **🐧 Linux** | [**Download Linux AppImage**](https://github.com/joravar/bapu-studio/releases/download/v1.1.0/Bapu-Studio-1.1.0-linux-x86_64.AppImage) <br> [Download Debian (.deb)](https://github.com/joravar/bapu-studio/releases/download/v1.1.0/Bapu-Studio-1.1.0-linux-amd64.deb) <br> [Download Tarball (.tar.gz)](https://github.com/joravar/bapu-studio/releases/download/v1.1.0/Bapu-Studio-1.1.0-linux-x64.tar.gz) | `.AppImage`, `.deb`, `.tar.gz` | ✅ Ready to run |

---

## ✨ What's New in v1.1.0

Bapu Studio v1.1.0 brings high-productivity database features inspired by DBeaver, enterprise SSL support, and testing engine integrations:

### 📑 1. DBeaver-Style Multi-Tab SQL Scratchpads
* **Multi-Buffer Workflow**: Open, manage, and switch between concurrent query tabs per database connection.
* **Inline Double-Click Renaming**: Rename tabs directly to organize complex engineering sessions (e.g., `Orders Analytics.sql`, `User Migration.sql`).
* **Persistent State**: Open tabs, orders, and editor contents are preserved across app restarts in isolated local storage.

### ⚡ 2. Execute Selection & Partial Queries (`Ctrl+Enter` / `F5`)
* Highlight any statement, CTE, or subquery in the editor to execute **only that selection**.
* Instant visual feedback badge confirms partial executions: `⚡ Running selection (48 chars)`.
* Full script execution is triggered whenever no selection is highlighted.

### 🧠 3. Dialect-Aware `⚡ Explain Plan` Visualizer
* One-click query plan generator supporting dialect-native execution plans:
  * **PostgreSQL**: `EXPLAIN (ANALYZE, COSTS, VERBOSE, BUFFERS, FORMAT JSON)`
  * **MySQL**: `EXPLAIN FORMAT=JSON`
  * **SQLite**: `EXPLAIN QUERY PLAN`
  * **MongoDB**: `.explain('executionStats')`
* Formatted execution plan output with a single-click **Copy Plan** utility.

### 📟 4. Dual Results View: Data Grid vs. Execution Console
* **`📊 Data Grid`**: Interactive table with dynamic column detection, CSV export, and live filtering.
* **`📟 Execution Console`**: Detailed execution telemetry including runtime latency (`ms`), rows affected, status codes, server notices, and full SQL statement audit.

### 🔒 5. Enterprise Root CA & Mutual TLS (mTLS) Drawer
* Built-in support for enterprise databases requiring custom TLS Authority (`ca.pem`), client certificates (`client-cert.pem`), and client private keys (`client-key.pem`).
* Compatible with AWS RDS (`global-bundle.pem`), Aiven, CockroachDB, and internal corporate PKI.
* Configurable strict certificate authority verification (`rejectUnauthorized`).

### 🧪 6. Postman Scripting Engine & Collection Migration
* **Scripting Engine**: Run pre-request and post-response assertion scripts using `pm.test`, `pm.environment.set`, `bapu.test`, and `bapu.expect`.
* **Collection Migration**: Full import/export support for Postman Collection v2.1 format and OpenAPI 3.0 (YAML & JSON) specifications.

### ⚡ 7. Real-Time Stream Studio
* Live inspection and bidirectional messaging for **WebSockets** and **Server-Sent Events (SSE)**.
* Frame logging, connection status telemetry, and payload inspection.

---

## 📊 Feature Comparison

| Feature | Postman | TablePlus | DBeaver CE | **Bapu Studio (v1.1.0 OSS)** |
| :--- | :---: | :---: | :---: | :---: |
| **Runtime Architecture** | Electron + Chromium | Native | JVM (bundled Java) | **Lean Electron (no JVM)** |
| **Memory Footprint** | Heavy at idle[^1] | Light | Heavy JVM overhead | **Lean by design (<150 MB typical idle)** |
| **100% Offline-First** | ❌ (Forced Cloud Sync) | ✅ | ✅ | **✅ (100% Local Storage)** |
| **Unified DB + API + Secrets** | ❌ | ❌ | ❌ | **✅ All-in-One Cockpit** |
| **Multi-Tab SQL Scratchpads** | ❌ None | ✅ | ✅ | **✅ Persistent Multi-Tab** |
| **Dialect EXPLAIN Visualizer** | ❌ None | ⚠️ Basic | ✅ Advanced | **✅ Native PG, MySQL, SQLite, Mongo** |
| **Custom Root CA & mTLS** | ⚠️ Partial | ⚠️ Paid | ✅ Complex | **✅ Built-in Certificate Drawer** |
| **Pre-Bundled DB Drivers** | ❌ None | ⚠️ Some | ✅ Most (JDBC) | **✅ Postgres, MySQL, Mongo, SQLite** |
| **WebSocket & SSE Streams** | ⚠️ Complex | ❌ None | ❌ None | **✅ Dedicated Stream Studio** |
| **Telemetry & Data Privacy** | Cloud telemetry | Closed source | Open source | **100% Zero Telemetry** |
| **License** | Proprietary ($$$) | Proprietary | Open Source (Apache 2.0) | **Open Source (AGPLv3)** |

[^1]: Based on publicly reported user observations, not a controlled benchmark. Check the [Releases page](https://github.com/joravar/bapu-studio/releases) for actual installer sizes.

---

## 🚀 Quick Start (Development)

### Prerequisites
* Node.js (v18+)
* npm (v9+)

### Run Standalone Desktop App
```bash
# 1. Clone the repository
git clone https://github.com/joravar/bapu-studio.git
cd bapu-studio

# 2. Install dependencies
npm install

# 3. Launch native desktop application
npm run desktop
```

### Run Web Browser Sandbox
```bash
# Start Vite local development server
npm run dev
```

### Run Comprehensive Test Suite
```bash
# Run all 49 full-lifecycle integration and parser tests
npm test
```

---

## 🏗️ Architecture & Technology Stack

* **Desktop Runtime:** Electron 34 with secure context-isolated preload bridge (`preload.cjs`).
* **Database Driver Engine:** Bundled pure-JS TCP drivers (`pg`, `mysql2`, `mongodb`, `sql.js`) with zero external native compilation dependencies.
* **Frontend Cockpit:** React 18 / TypeScript 5 with ultra-fast Vite 6 bundler.
* **UI Design System:** Zero CSS framework overhead — hand-crafted vanilla HSL dark-mode theme.
* **Storage Engine:** 100% local persistence across cold reboots with zero cloud leaks.
* **Resilience:** Defensive state sanitization and zero-crash React Error Boundary.

---

## 🤖 AI Copilot — Current Status

The AI Copilot panel (available in both API Studio and Database Studio) ships today as a **local, fully offline heuristic engine**. It pattern-matches your prompt against a small set of built-in SQL and JSON templates and returns a result instantly — no network call is made, which is why it's genuinely zero-telemetry: there's nothing to transmit yet.

The provider selector (Local Ollama / OpenAI BYOK / Anthropic BYOK) and API key field are present in the UI ahead of real model-backed generation, which is still on the roadmap. Selecting a provider or entering a key does not currently change the output — treat the Copilot as a smart snippet generator for now, not a live LLM integration.

We'd rather say this plainly here than have it be a surprise. If you want to help wire up real Ollama/OpenAI/Anthropic calls, contributions are welcome.

---

## 💼 Commercial Open Source Model (COSS)
 
* **Community Edition (AGPLv3):** 100% free for individual developers, local offline use, and open-source projects.
* **Bapu Pro & Team Editions:** Optional end-to-end encrypted cloud vault sync, multi-device pairing, and team SSO.

---

## ⚖️ Legal, Trademark & Liability Disclaimers

* **Nominative Fair Use:** Postman®, TablePlus®, DBeaver®, Doppler®, PostgreSQL®, MySQL®, MongoDB®, Redis®, Docker®, Electron®, and other third-party product names, trademarks™, or registered® trademarks mentioned in documentation, benchmark charts, or comparison tables are the property of their respective owners. Their reference here is purely for nominative, educational, and descriptive comparison purposes and does not imply any affiliation, endorsement, partnership, or sponsorship by their respective holders.
* **100% Clean-Room Independent Implementation:** All code, architecture, and UI designs in this repository are independent, original clean-room works authored specifically for Bapu Studio and distributed under the **GNU Affero General Public License v3.0 (AGPLv3)**.
* **No Warranty ("AS IS"):** In accordance with Sections 15 and 16 of the GNU AGPLv3 license, this software is provided on an *"AS IS"* and *"AS AVAILABLE"* basis without warranties of any kind, either express or implied, including but not limited to fitness for a particular purpose, merchantability, or non-infringement.
* **Database & Query Execution Responsibility:** Users are solely responsible for reviewing and verifying all SQL/NoSQL queries, migrations, and database operations before executing them against production or critical database servers. The authors of Bapu Studio accept no liability for data loss or service disruption.
* **AI Copilot Output:** The AI Copilot currently runs as a local template/heuristic engine (see [AI Copilot — Current Status](#-ai-copilot--current-status)), not a live model integration. Its output is a starting-point suggestion only — users must inspect and validate all generated queries and payloads before execution.
* **Privacy & Local Storage:** Bapu Studio Community Edition operates 100% locally. Connection strings, API secrets, and queries are stored on the user's local machine and are never transmitted to any third-party telemetry server.

---

## 💖 Support & GitHub Sponsors

Bapu Studio is an open-core, community-driven developer tool. You can sponsor the ongoing maintenance and development of Bapu Studio on [GitHub Sponsors](https://github.com/sponsors/joravar):

| Tier | Monthly | Perks |
| :--- | :---: | :--- |
| **☕ Backer** | **$5/mo** | Listed on the README sponsors wall & Discord Supporter badge |
| **🚀 Pro Sponsor** | **$12/mo** | Pro Supporter badge, early access preview of upcoming Bapu Pro features (cloud sync relay & AI model integrations), & direct roadmap input |
| **🏢 Corporate Silver** | **$100/mo** | Company logo + do-follow link on GitHub README & docs website |
| **👑 Corporate Gold** | **$500/mo** | Top-tier banner logo on README, priority bug fixes & feature requests |

[👉 **Click here to Sponsor Bapu Studio on GitHub**](https://github.com/sponsors/joravar)

---

## 📄 License
This project is licensed under the **GNU Affero General Public License v3.0 (AGPLv3)**. Commercial team licenses are available.
