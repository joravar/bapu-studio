# ⚡ Bapu Studio — The Honest Open-Source Developer Cockpit

<div align="center">

[![License: AGPL v3](https://img.shields.io/badge/License-AGPLv3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Bapu Studio CI](https://github.com/joravar/bapu-studio/actions/workflows/ci.yml/badge.svg)](https://github.com/joravar/bapu-studio/actions/workflows/ci.yml)
[![Release: v1.4.0](https://img.shields.io/badge/Release-v1.4.0-emerald.svg)](https://github.com/joravar/bapu-studio/releases/tag/v1.4.0)
[![Platforms](https://img.shields.io/badge/Platforms-Windows%20%7C%20macOS%20%7C%20Linux-informational.svg)](https://github.com/joravar/bapu-studio/releases/tag/v1.4.0)
[![Website](https://img.shields.io/badge/Website-Live-brightgreen.svg)](https://joravar.github.io/bapu-studio/)
[![Sponsor](https://img.shields.io/badge/GitHub-Sponsor-pink.svg)](https://github.com/sponsors/joravar)

**The honest, lightweight, local-first developer cockpit combining an API Client, Multi-Tab Database Studio, Real-Time Streams, and Secrets Matrix into one seamless desktop application.**

[📦 Downloads](#-downloads) • [✨ New in v1.4.0](#-whats-new-in-v140) • [🌐 Live Website](https://joravar.github.io/bapu-studio/) • [📊 Comparison](#-feature-comparison) • [🏗️ Architecture](#️-architecture--technology-stack) • [🤖 AI Copilot](#-ai-copilot--current-status) • [📡 Stream Studio Status](#-stream-studio--current-status) • [📊 Data Grid Status](#-data-grid--current-status) • [💖 Sponsor](#-support--github-sponsors) • [📄 License](#-license)

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
│  • Pre/Post Script Engine    │  • Postgres/MySQL/Mongo/Redis│  • Variable Interpolation         │
│  • Postman v2.1 & OpenAPI    │  • ⚡ Query EXPLAIN Plans    │  • WebSocket & SSE Stream Studio  │
│  • Sub-ms Latency Benchmarks │  • Dual Grid / Console Mode  │  • Custom Root CA & mTLS Support  │
└──────────────────────────────┴──────────────────────────────┴───────────────────────────────────┘
```

---

## 📦 Downloads

Direct native installer packages are available on [GitHub Releases (v1.4.0)](https://github.com/joravar/bapu-studio/releases/tag/v1.4.0):

| Operating System | Installer Package | Format | Status |
| :--- | :--- | :--- | :--- |
| **🍏 macOS (Apple Silicon)** | [**Download macOS DMG**](https://github.com/joravar/bapu-studio/releases/download/v1.4.0/Bapu-Studio-1.4.0-mac-arm64.dmg) <br> [Download macOS Zip](https://github.com/joravar/bapu-studio/releases/download/v1.4.0/Bapu-Studio-1.4.0-mac-arm64.zip) | `.dmg` & `.zip` | ✅ Signed & Notarized by Apple |
| **🪟 Windows** | [**Download Windows Installer**](https://github.com/joravar/bapu-studio/releases/download/v1.4.0/Bapu-Studio-Setup-1.4.0.exe) <br> [Download Portable .exe](https://github.com/joravar/bapu-studio/releases/download/v1.4.0/Bapu-Studio-Portable-1.4.0.exe) <br> [Download Windows Zip](https://github.com/joravar/bapu-studio/releases/download/v1.4.0/Bapu-Studio-1.4.0-win-x64.zip) | `.exe` (NSIS & Portable) | ✅ Ready to install / run |
| **🐧 Linux** | [**Download Linux AppImage**](https://github.com/joravar/bapu-studio/releases/download/v1.4.0/Bapu-Studio-1.4.0-linux-x86_64.AppImage) <br> [Download Debian (.deb)](https://github.com/joravar/bapu-studio/releases/download/v1.4.0/Bapu-Studio-1.4.0-linux-amd64.deb) <br> [Download Tarball (.tar.gz)](https://github.com/joravar/bapu-studio/releases/download/v1.4.0/Bapu-Studio-1.4.0-linux-x64.tar.gz) | `.AppImage`, `.deb`, `.tar.gz` | ✅ Ready to run |

---

## ✨ What's New in v1.4.0

* **Data Grid edits now use a real commit/rollback model** (DBeaver-style), instead of writing on every keystroke:
  * Cell edits, **Add Row**, and **Delete Selected** all stage locally first — edited cells highlight amber, staged new rows show green, rows marked for deletion show struck through with an undo button.
  * A **"N pending changes"** banner appears with **Save Changes** / **Revert**. Save runs every staged change as **one real transaction** (`BEGIN`/`COMMIT`/`ROLLBACK`) for Postgres, MySQL, and SQLite, or a MongoDB session transaction where the server supports one (every MongoDB Atlas cluster does; a standalone `mongod` does not).
  * On a standalone MongoDB server, the batch honestly reports that it applied sequentially rather than atomically (`atomic: false`), instead of implying an all-or-nothing guarantee that engine can't actually provide.
  * Switching tables, connections, or running a new query while changes are pending now asks for confirmation first.
* **Fix**: cell editing wasn't obviously discoverable — there was only a hover tooltip. The toolbar now shows a persistent "Double-click a cell to edit" hint whenever editing is available.

---

## ✨ What's New in v1.3.0

* **Editable Data Grid** — double-click a cell to edit it, plus Add Row and Delete Selected, for Postgres/MySQL/SQLite/MongoDB. All generated SQL binds values as real query parameters, never string-concatenated. Editing is auto-disabled (with a **Read-only** indicator and reason) for joins/aggregates, tables without a primary key, the built-in demo data, and Redis — see [Data Grid — Current Status](#-data-grid--current-status).
* **Dark / Light theme toggle** — the sun/moon button in the header switches themes instantly, persisted locally.
* **Real app logo** — the actual Bapu Studio mark now appears in the app header and on this project's website, replacing a placeholder.
* **UX fix**: the SQLite drag-and-drop box no longer permanently occupies sidebar space once a connection is active — it collapses to a small "Load a SQLite file..." link.
* **Bug fix**: a SQL query that legitimately matched zero rows could show a fabricated "Command executed successfully" message on SQLite instead of a real, empty result — fixed and covered by a regression test.

---

## ✨ What's New in v1.2.1

A maintenance release: every dependency brought up to its true current latest, most notably Electron itself, which had drifted 10 majors behind and out of upstream's supported/security-patched range.

* **Electron 34 → 44** — was on a version no longer receiving security patches; now on current stable.
* **React 18 → 19**, **TypeScript 5.7 → 7.0** (new native compiler), **Vite 6 → 8**, **@vitejs/plugin-react 4 → 6**, **lucide-react 0.475 → 1.47**, plus routine bumps to `mongodb`, `mysql2`, and dev tooling.
* No feature or behavior changes. Verified via the full test suite, a clean production build, and end-to-end checks against the real packaged Electron app: encrypted-secrets round-trip, the sandboxed script-test-engine isolation, native dialogs, and the real SQLite engine all still behave correctly on the new Electron/Chromium/Node runtime.

---

## ✨ What's New in v1.2.0

Bapu Studio v1.2.0 brings high-productivity database features inspired by DBeaver, enterprise SSL/SSH connectivity, a real Redis driver, a real SQLite engine, git-friendly collection storage, and testing engine integrations:

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
* Live inspection and bidirectional messaging for real **WebSocket** and **Server-Sent Events (SSE)** connections — genuine `EventSource`/`WebSocket` sockets, not a simulation.
* Frame logging, per-message latency, and connection status telemetry (connecting / connected / error / closed).

### 🔑 8. SSH Tunnel (Bastion Host) Support
* Connect to Postgres, MySQL, MongoDB, or Redis instances that are only reachable through a bastion/jump host — the database connection is forwarded through an SSH session opened first.
* Password or private-key (with optional passphrase) authentication to the SSH host.
* Applies to discrete host/port connections; not combined with a raw connection-string URI, since a connection string bakes in the real target host directly.

### 📁 9. Git-Friendly Collection Storage
* Link any API collection to a folder on disk — Bapu Studio mirrors it as plain JSON, **one file per request** plus a small manifest, instead of one opaque blob.
* One-file-per-request means editing a single request produces a small, reviewable diff, and two people editing different requests in the same collection don't collide — the exact pain point Postman-style single-file collections have in git.
* **Load Collection from Folder** reads a linked collection back in — the way a teammate picks up a collection after `git pull`/`git clone` rather than needing an explicit import/export round-trip.
* Changes sync to disk automatically (debounced) while a collection stays linked; unlinking stops the sync without touching files already written.

### 🧵 10. Redis Support
* Full Redis driver alongside Postgres/MySQL/MongoDB — connection string or discrete host/port, optional TLS, optional username+password (Redis 6+ ACL).
* Run any raw Redis command in the SQL Studio editor (`GET`, `HGETALL`, `LRANGE`, `SCAN`, ...) — results are normalized into the same grid view used for SQL/Mongo results.
* The schema browser has no tables to show for a flat keyspace, so it groups a keyspace sample by Redis type instead (`string`, `hash`, `list`, ...) and clicking one runs the right read command against a real sample key.
* Works through the SSH Tunnel and Custom CA drawers described above, same as the other drivers.

### 💾 11. Real SQLite Engine
* Dropping a `.sqlite`/`.db` file now actually reads it — real schema (`sqlite_master` + `PRAGMA table_info`), real row counts, and real query execution/writes via the bundled `sql.js` WASM engine, not a hardcoded placeholder schema.
* A SQLite connection lives for the current app session (the WASM database instance can't be serialized into local storage) — re-drop the file after restarting the app to reconnect.

---

## 📊 Feature Comparison

| Feature | Postman | TablePlus | DBeaver CE | PostPilot | **Bapu Studio (v1.2.0 OSS)** |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **Runtime Architecture** | Electron + Chromium | Native | JVM (bundled Java) | Native | **Lean Electron (no JVM)** |
| **Memory Footprint** | Heavy at idle[^1] | Light | Heavy JVM overhead | Light | **Lean by design (<150 MB typical idle)** |
| **100% Offline-First** | ❌ (Forced Cloud Sync) | ✅ | ✅ | ✅ | **✅ (100% Local Storage)** |
| **Unified DB + API** | ❌ | ❌ | ❌ | ✅ | **✅ All-in-One Cockpit** |
| **Built-in Secrets/.env Vault** | ❌ | ❌ | ❌ | ❌ | **✅ Secrets Matrix** |
| **Multi-Tab SQL Scratchpads** | ❌ None | ✅ | ✅ | ⚠️ Basic | **✅ Persistent Multi-Tab** |
| **Dialect EXPLAIN Visualizer** | ❌ None | ⚠️ Basic | ✅ Advanced | ❌ None | **✅ Native PG, MySQL, SQLite, Mongo** |
| **Custom Root CA & mTLS** | ⚠️ Partial | ⚠️ Paid | ✅ Complex | ⚠️ Unclear | **✅ Built-in Certificate Drawer** |
| **SSH Tunnel (Bastion Host)** | ❌ None | ✅ | ✅ | ⚠️ Unclear | **✅ Password or Private Key Auth** |
| **Git-Diffable Collection Storage** | ❌ Single-file export only | N/A (not an API client) | N/A | ❌ Not advertised | **✅ One file per request, auto-synced** |
| **AI SQL/Payload Assistant** | ✅ Postbot (cloud, account required) | ❌ None | ✅ Free in CE (GPT-5 / Copilot, late 2025+) | ❌ Not advertised | **✅ Local Ollama + OpenAI/Anthropic BYOK** |
| **WebSocket & SSE Streams** | ⚠️ Complex | ❌ None | ❌ None | ❌ None | **✅ Dedicated Stream Studio (live sockets)** |
| **Telemetry & Data Privacy** | Cloud telemetry | Closed source | Open source | Closed source | **100% Zero Telemetry** |
| **License / Pricing** | Proprietary ($$$/mo) | Proprietary | Open Source (Apache 2.0) | Proprietary ($40 one-time + $20/yr updates) | **Open Source (AGPLv3)** |

[^1]: Based on publicly reported user observations, not a controlled benchmark. Check the [Releases page](https://github.com/joravar/bapu-studio/releases) for actual installer sizes.

PostPilot in particular is the closest match to Bapu Studio's own pitch — a local-first API + database client in one app — so it's included here directly rather than left out. The gaps we believe still matter: PostPilot is a paid, closed-source, single-developer license with no secrets vault and no streaming support at all, while Bapu Studio is AGPLv3, free, and adds a dedicated Secrets Matrix plus a real WebSocket/SSE Stream Studio.

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
# Run all full-lifecycle integration and parser tests, including a real sql.js SQLite engine suite
npm test
```

### Run the Real Database Driver Integration Tests (optional)
```bash
# Each of these downloads and boots a real, disposable database server (PostgreSQL, MySQL, or
# MongoDB) to test the actual driver end-to-end — real auth failures, real schema introspection,
# real query execution. Not part of `npm test` / CI (real server binaries add real time/network
# weight across a 9-way OS/Node matrix) — run the relevant one when touching that driver's logic.
npm run test:postgres
npm run test:mysql
npm run test:mongodb

# SQLite doesn't need a separate opt-in script — it runs the real sql.js WASM engine as part of
# `npm test` (no server to boot). Redis was verified the same way (a real embedded server) during
# development but doesn't have a standing script here yet.
```

---

## 🏗️ Architecture & Technology Stack

* **Desktop Runtime:** Electron 44 with secure context-isolated preload bridge (`preload.cjs`).
* **Database Driver Engine:** Bundled pure-JS TCP drivers (`pg`, `mysql2`, `mongodb`, `ioredis`, `sql.js`) with zero external native compilation dependencies.
* **Frontend Cockpit:** React 19 / TypeScript 7 with ultra-fast Vite 8 bundler.
* **UI Design System:** Zero CSS framework overhead — hand-crafted vanilla HSL theme with a dark/light toggle (the sun/moon button in the header), persisted locally.
* **Storage Engine:** 100% local persistence across cold reboots with zero cloud leaks.
* **Resilience:** Defensive state sanitization and zero-crash React Error Boundary.

---

## 🤖 AI Copilot — Current Status

The AI Copilot panel (available in both API Studio and Database Studio) now makes **real, live model calls** for all three providers:

* **Local Ollama** (`localhost:11434`, model `llama3`) — used first when selected, with a 4s timeout.
* **OpenAI BYOK** — calls `gpt-4o-mini` directly with the API key you enter, stored locally.
* **Anthropic BYOK** — calls `claude-3-5-sonnet` directly with the API key you enter, stored locally.

Your database schema (table/column names — never row data, credentials, or connection strings) is sent as context when a cloud provider (OpenAI/Anthropic) is selected, so the model can generate schema-accurate SQL. Nothing is sent when using Local Ollama.

If a selected provider is unreachable, times out, or has no API key entered, the Copilot **falls back to a local, fully offline heuristic engine** that pattern-matches your prompt against built-in SQL/JSON templates — so it never simply fails silently, but the output quality in that fallback case is template-based, not model-generated. API keys you enter are encrypted at rest via the OS keychain (Electron `safeStorage`), not stored in plaintext.

If you want to help extend provider support (e.g. more models, streaming responses), contributions are welcome.

---

## 📡 Stream Studio — Current Status

Stream Studio opens genuine connections: `EventSource` for SSE mode and `WebSocket` for WS mode, against whatever URL you enter. Messages shown in the timeline are real server events/frames with real receive latency, not scripted playback. Switching between SSE/WS modes, disconnecting, and closing the app all cleanly tear down the underlying connection.

A couple of things worth knowing:
* **CORS applies to SSE** the same way it does to `fetch` — if a remote SSE endpoint doesn't send permissive CORS headers, the browser will block the response and you'll see a connection error, even though the server is reachable.
* When a server ends an SSE stream normally (rather than the client disconnecting), the browser's `EventSource` API reports that the same way it reports a network error — so a "stream error or connection closed" message doesn't necessarily mean something went wrong.

---

## 📊 Data Grid — Current Status

The Data Grid (Database Studio's results view) supports real **inline editing** for Postgres, MySQL, SQLite, and MongoDB, using a DBeaver-style **pending changes → Save/Revert** model rather than writing on every keystroke:

* **Edit a cell** — double-click any non-key cell; Enter (or clicking away) stages the new value locally, shown with an amber highlight. Nothing is written yet. Clearing a cell stages it to `NULL`.
* **Add Row** — a form built from the table's real column list (a JSON document editor for MongoDB, since its schema is flexible) stages a new row, shown at the bottom of the grid with a green tint until saved.
* **Delete Selected** — check one or more rows to stage them for deletion; they're shown struck through with an undo (↩) button, not actually removed until saved.
* **Save Changes (N)** — runs every staged edit/insert/delete as **one all-or-nothing batch**: a real transaction (`BEGIN`/`COMMIT`/`ROLLBACK`) for Postgres, MySQL, and SQLite, or a MongoDB session transaction when the server supports one (every MongoDB Atlas cluster, including the free tier, does — a standalone/local `mongod` does not). If a mutation partway through the batch fails, the whole transaction rolls back rather than leaving a partial edit.
* **Revert** — discards every staged change with no database access at all.
* Running a new query, switching tables, or switching connections while changes are pending asks for confirmation first, so they're never silently discarded.

**Honesty note on MongoDB:** a standalone MongoDB server (not a replica set) cannot do multi-document transactions at all. Bapu Studio detects this and falls back to applying the batch sequentially — but it reports `atomic: false` and says so in the history log and any error, rather than the UI implying the same all-or-nothing guarantee it gives for Postgres/MySQL/SQLite when that guarantee isn't actually true for that server.

Editing is only offered when it's actually safe to target a specific row, and shows a **Read-only** indicator (with the reason on hover) otherwise:
* The result on screen has to be a plain browse of one table (`SELECT * FROM that_table` / `collection.find(...)`) — not a JOIN, aggregate, or hand-written query that merely happens to share column names, since a save writes an `UPDATE`/`DELETE` back at that exact table.
* The table needs a real primary key (or `_id` for MongoDB) present in the result columns, so a mutation can never guess at which row to hit.
* The built-in sample/demo database is intentionally read-only — there's no real database behind it to write to, so editing it is refused with a clear message rather than silently pretending to succeed.
* **Redis stays excluded** — its keyspace browser isn't a row-based table with a stable primary key to edit against.

All generated SQL binds staged values as real query parameters (`$1`/`?` placeholders), never string-concatenated, so typed cell values can't be interpreted as SQL.

---

## 💼 Commercial Open Source Model (COSS)
 
* **Community Edition (AGPLv3):** 100% free for individual developers, local offline use, and open-source projects.
* **Bapu Pro & Team Editions:** Optional end-to-end encrypted cloud vault sync, multi-device pairing, and team SSO.

---

## ⚖️ Legal, Trademark & Liability Disclaimers

* **Nominative Fair Use:** Postman®, TablePlus®, DBeaver®, PostPilot®, Doppler®, PostgreSQL®, MySQL®, MongoDB®, Redis®, Docker®, Electron®, and other third-party product names, trademarks™, or registered® trademarks mentioned in documentation, benchmark charts, or comparison tables are the property of their respective owners. Their reference here is purely for nominative, educational, and descriptive comparison purposes and does not imply any affiliation, endorsement, partnership, or sponsorship by their respective holders.
* **100% Clean-Room Independent Implementation:** All code, architecture, and UI designs in this repository are independent, original clean-room works authored specifically for Bapu Studio and distributed under the **GNU Affero General Public License v3.0 (AGPLv3)**.
* **No Warranty ("AS IS"):** In accordance with Sections 15 and 16 of the GNU AGPLv3 license, this software is provided on an *"AS IS"* and *"AS AVAILABLE"* basis without warranties of any kind, either express or implied, including but not limited to fitness for a particular purpose, merchantability, or non-infringement.
* **Database & Query Execution Responsibility:** Users are solely responsible for reviewing and verifying all SQL/NoSQL queries, migrations, and database operations before executing them against production or critical database servers. The authors of Bapu Studio accept no liability for data loss or service disruption.
* **AI Copilot Output:** The AI Copilot makes live calls to the model provider you select (see [AI Copilot — Current Status](#-ai-copilot--current-status)) and falls back to a local template engine if that call fails. Its output — from either path — is a starting-point suggestion only; users must inspect and validate all generated queries and payloads before execution.
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
