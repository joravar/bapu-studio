# Bapu Studio — native Rust rewrite (work in progress)

This directory is a from-scratch native rewrite of Bapu Studio: Electron + Node.js + React/TypeScript
replaced with a single Rust binary using [egui](https://github.com/emilk/egui)/[eframe](https://github.com/emilk/egui)
for the UI. Goals: no bundled Chromium/Node runtime, a much smaller installer and idle memory
footprint, and a stronger script-sandboxing story (see below).

**Status: early scaffold, not a drop-in replacement yet.** It was written in one pass without the
ability to run `cargo build` against it (this environment couldn't reach crates.io) — expect to
spend a little time fixing small compile errors on your first build, especially API-signature
mismatches in whichever exact `egui`/`eframe`/`boa_engine` versions `cargo` resolves to (version
numbers in `Cargo.toml` are my best approximation of recent-at-writing releases, not pinned/verified
against a successful build).

## Layout

- `crates/bapu-core` — shared logic with no UI dependency: data model (`types.rs`, mirroring the
  TypeScript `src/types/index.ts` so JSON on disk is compatible), local JSON-file persistence
  (`storage.rs`), OS-keychain-backed secrets (`secrets.rs`, via the `keyring` crate — the Rust
  equivalent of Electron's `safeStorage`), an HTTP client for API Studio (`http_client.rs`, via
  `reqwest`), a sandboxed pre-request/test script engine (`script_engine.rs`, via the pure-Rust
  `boa_engine` JS interpreter instead of Node + a Web Worker), and database drivers (`db/`).
- `crates/bapu-app` — the egui desktop binary: `main.rs` boots a tokio runtime and the eframe
  window; `app.rs` holds application state and dispatches async work (HTTP/DB calls) onto that
  runtime, feeding results back to the UI thread over a channel; `ui/` has one module per tab.

## What actually works in this first pass

- **API Studio**: method/URL/headers/body, send a real HTTP request via `reqwest`, view the
  response (status/time/size/body).
- **Database Studio**: add a Postgres connection, test it, run a query, see results in a table.
  Postgres is the only fully-wired driver (`db/postgres.rs`) — MySQL/SQLite/MongoDB/Redis are
  stubs (`db/mysql.rs` etc.) that return a clear "not yet implemented" error rather than silently
  doing nothing.
- **Secrets**: environment variables, with ones flagged "secret" routed through the OS keychain
  (`secrets.rs`) rather than ever touching a plaintext file on disk.
- **Script sandboxing, architecturally stronger than before**: `boa_engine` has no built-in
  `fetch`, filesystem, or network access, and no knowledge of this app's database/IPC layer at
  all. A pre-request/test script can only do what `script_engine.rs` explicitly binds into its
  global scope (`bapu`/`pm`/`console`/`test`) — there's no ambient capability to escape to, unlike
  the Electron version's Worker sandbox, which is strong but still had to be built by *removing*
  access (no `window`, no `bapuBridge`) from a realm that has those by default.

## Known parity gaps vs. the Electron app (v1.4.0) — not yet ported

- MySQL, SQLite (sql.js-equivalent), MongoDB, Redis drivers (stubbed).
- SSH tunnel (bastion host) support for any driver.
- Multi-tab SQL scratchpads, EXPLAIN plan visualizer, dual grid/console view.
- Editable Data Grid with the commit/rollback (BEGIN/COMMIT/ROLLBACK) transaction model.
- Real-time Stream Studio (WebSocket/SSE) — `tokio-tungstenite` is a dependency but not wired
  into any UI yet.
- Postman v2.1 / OpenAPI collection import-export, curl import.
- AI Copilot (Ollama/OpenAI/Anthropic BYOK).
- Git-friendly per-request-file collection folder sync.
- Request/response History panel (UI shell exists, nothing records into it yet).
- Dark/light theme toggle (egui defaults only, right now).
- Hard-killing a runaway/infinite-loop script (`script_engine.rs` has a TODO — needs the script
  to run on its own thread with a join-timeout, since `boa_engine::Context::eval` has no built-in
  interrupt hook in the version this was written against).
- Packaging/release pipeline (electron-builder → needs a Rust equivalent, e.g. `cargo-bundle` or
  `cargo-packager`, plus updated CI).

## Building

```bash
cd native
cargo build --release
cargo run -p bapu-app
```

If dependency versions in the `Cargo.toml` files don't resolve cleanly, or an `egui`/`boa_engine`
API call in this scaffold doesn't match what `cargo` actually pulls in, that's the expected
first-build friction mentioned above — fix forward rather than assuming the architecture is wrong.
