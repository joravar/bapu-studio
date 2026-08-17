# ⚡ Bapu Studio — The Honest Open-Source Developer Cockpit

<div align="center">

[![License: AGPL v3](https://img.shields.io/badge/License-AGPLv3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Bapu Studio CI](https://github.com/joravar/bapu-studio/actions/workflows/ci.yml/badge.svg)](https://github.com/joravar/bapu-studio/actions/workflows/ci.yml)
[![Platforms](https://img.shields.io/badge/Platforms-Windows%20%7C%20macOS-informational.svg)](https://github.com/joravar/bapu-studio/releases/tag/v1.0.0)
[![Website](https://img.shields.io/badge/Website-Live-brightgreen.svg)](https://joravar.github.io/bapu-studio/)
[![Sponsor](https://img.shields.io/badge/GitHub-Sponsor-pink.svg)](https://github.com/sponsors/joravar)

**The honest, lightweight, local-first developer cockpit combining an API Client, Database Studio, and Secrets Matrix into one seamless desktop application.**

[📦 Downloads](#-downloads) • [🌐 Live Website](https://joravar.github.io/bapu-studio/) • [🏗️ Architecture](#️-architecture--technology-stack) • [🤖 AI Copilot](#-ai-copilot--current-status) • [💖 Sponsor](#-support--github-sponsors) • [📄 License](#-license)

</div>

---

## 🌟 Why Bapu Studio?

Developers are tired of running several heavy apps just to test an endpoint, run a database query, and check a `.env` secret.

**Bapu Studio** stands for **simplicity, truth, zero bloat, and complete local privacy**.

```
┌────────────────────────────────────────────────────────────────────────┐
│                              BAPU STUDIO                               │
├───────────────────────┬────────────────────────┬───────────────────────┤
│    🚀 API CLIENT      │   🗄️ DATABASE STUDIO   │   🔐 SECRETS MATRIX   │
│  • REST, GraphQL      │  • Postgres, MySQL     │  • Scoped .env Matrix │
│  • Latency & Headers  │  • MongoDB & SQLite    │  • Variable Inject    │
│  • cURL Import/Export │  • Dynamic Table Grid  │  • Secret Masking     │
└───────────────────────┴────────────────────────┴───────────────────────┘
```

---

## 📦 Downloads

Direct native installer packages are available on [GitHub Releases](https://github.com/joravar/bapu-studio/releases/tag/v1.0.0):

| Operating System | Installer Package | Format |
| :--- | :--- | :--- |
| **🪟 Windows** | [**Download Windows Installer**](https://github.com/joravar/bapu-studio/releases/tag/v1.0.0) | `.exe` (NSIS & Portable) |
| **🍏 macOS (Apple Silicon)** | [**Download macOS DMG**](https://github.com/joravar/bapu-studio/releases/tag/v1.0.0) | `.dmg` & `.zip` |

---

## 📊 Feature Comparison

| Feature | Postman | TablePlus | DBeaver CE | **Bapu Studio (OSS)** |
| :--- | :---: | :---: | :---: | :---: |
| **Runtime** | Electron + Chromium | Native | JVM (bundled Java) | Electron (no JVM) |
| **Idle RAM** | Widely reported in the hundreds of MB, climbing over long sessions[^1] | Generally light — native app | Can run heavy due to JVM overhead | Lean by design — no JVM, minimal UI framework |
| **100% Offline-First** | ❌ (Forced Cloud) | ✅ | ✅ | **✅ (100% Local Storage)** |
| **Unified DB + API + Secrets** | ❌ | ❌ | ❌ | **✅ All-in-One** |
| **Pre-Bundled Drivers** | ❌ None | ⚠️ Some | ✅ Most (JDBC) | **✅ Postgres, MySQL, Mongo, SQLite** |
| **Telemetry / Data Privacy** | Cloud sync by default | Closed source, local | Open source, local | **100% Zero Telemetry** |
| **License** | Proprietary ($$$) | Proprietary | Open Source (Apache 2.0) | **Open Source (AGPLv3)** |

[^1]: Based on publicly reported user issues, not a controlled benchmark. We haven't published exact installer size or RAM numbers for Bapu Studio yet — figures vary by OS and version, and we'd rather not quote a number we haven't measured. Check the [Releases page](https://github.com/joravar/bapu-studio/releases) for actual installer sizes.

---

## 🚀 Quick Start (Development)

### Prerequisites
* Node.js (v18+)

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

---

## 🏗️ Architecture & Technology Stack

* **Desktop Engine:** Electron runtime with bundled pure-JS TCP drivers (`pg`, `mysql2`, `mongodb`, `sql.js`).
* **Frontend Cockpit:** React 18 / TypeScript with Vite compiler.
* **Design System:** Tailored HSL Dark Theme Vanilla CSS (Zero CSS framework bloat).
* **Storage Engine:** 100% local persistence across cold boots.

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
| **🚀 Pro Sponsor** | **$12/mo** | Instant **Bapu Pro License Key** (Multi-device sync & early access to AI Copilot model integrations) |
| **🏢 Corporate Silver** | **$100/mo** | Company logo + do-follow link on GitHub README & docs website |
| **👑 Corporate Gold** | **$500/mo** | Top-tier banner logo on README, priority bug fixes & feature requests |

[👉 **Click here to Sponsor Bapu Studio on GitHub**](https://github.com/sponsors/joravar)

---

## 📄 License
This project is licensed under the **GNU Affero General Public License v3.0 (AGPLv3)**. Commercial team licenses are available.
