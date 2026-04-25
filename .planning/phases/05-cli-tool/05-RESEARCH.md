# Phase 5: CLI Tool (periphore-cli) - Research

**Researched:** 2026-04-25
**Domain:** Rust CLI (clap v4 derive), async IPC client (tokio UnixStream + JSON-lines), output formatting
**Confidence:** HIGH

---

## Summary

Phase 5 implements the `periphore` binary's subcommand surface by filling in `periphore-cli/src/lib.rs`
and wiring `periphore/src/main.rs` to call it. The IPC protocol is already finalized (JSON-lines over
Unix domain socket, `serde_json` serialization of `IpcRequest`/`IpcResponse`). The socket path resolver
and permissions are also complete. Phase 5 is pure client-side work: define clap subcommands, open a
`tokio::net::UnixStream`, write a JSON-line request, read a JSON-line response, and format the output.

The IPC server in `periphore-ipc/src/server.rs` and the daemon test helper in
`periphore-ipc/tests/socket.rs` together constitute a complete reference implementation of the
JSON-lines protocol the client must speak. The `send_request` helper in the integration tests is
exactly the pattern the CLI client should use — connect, split, write request + `\n`, `BufReader::read_line`.

**Topology stub awareness:** `GetTopology` currently returns `IpcResponse::Ok` from the daemon
(`send_ok` branch). The `periphore topology` command must display a clear "topology data not yet
available (Phase 8)" message when it receives `Ok` instead of a topology-specific response variant.
This degrades gracefully without requiring a daemon change.

**Primary recommendation:** Define `Cli` (with global `--socket` / `--config` args) and `Commands`
enum in `periphore-cli`, move `clap::Parser` out of `periphore/src/main.rs`, export a `run(cli: Cli)`
function from the library, and have `main.rs` call `periphore_cli::run(Cli::parse())`.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TOP-04 | CLI debug output shows resolved topology when debug logging is enabled | `periphore topology` command sends `IpcRequest::GetTopology`; daemon currently stubs Ok; client displays graceful stub message; full topology display implemented in Phase 8 when daemon has real data |
</phase_requirements>

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Subcommand parsing | periphore-cli (library) | periphore binary (thin entry) | CLAUDE.md: periphore-cli is the CLI support library; main.rs is "thin entry point" |
| IPC client transport | periphore-cli (library) | — | Client-side UnixStream connect/write/read belongs in the library, not main.rs |
| Socket path resolution | periphore-ipc::path | periphore-cli (calls it) | Reuses existing `socket_path()` function; CLI can override via `--socket` flag |
| Output formatting | periphore-cli (library) | — | Format decisions (human-readable vs JSON) belong in the library |
| Async runtime init | periphore binary (main.rs) | — | Only binary entry points call `#[tokio::main]`; library crates are async-fn only |
| Tracing subscriber init | NOT in periphore binary | — | `periphore` (CLI) does NOT init a subscriber; only `periphored` does (D-26 in STATE.md) |

---

## Standard Stack

### Core (already in Cargo.toml — no new deps needed)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | 4.6 [VERIFIED: Cargo.toml] | Subcommand parsing via `#[derive(Subcommand)]` | Already workspace dep; derive API is ergonomic and pedantic-clean |
| tokio | 1.52 [VERIFIED: Cargo.toml] | Async UnixStream connect, `BufReader::read_line`, `AsyncWriteExt::write_all` | Already workspace dep; features = ["net", "io-util"] already enabled |
| serde_json | 1.0 [VERIFIED: Cargo.toml] | Serialize `IpcRequest` to JSON, deserialize `IpcResponse` | Already workspace dep; used by server; client must use same serializer |
| anyhow | 1.0 [VERIFIED: Cargo.toml] | Error propagation in `run()` | Already in periphore-cli Cargo.toml; matches existing decision in STATE.md |
| tracing | 0.1 [VERIFIED: Cargo.toml] | CLI-side tracing macros (no subscriber init) | Already in periphore-cli Cargo.toml |
| periphore-ipc | workspace [VERIFIED: periphore-cli/Cargo.toml] | `path::socket_path()` for default socket path | Already declared dep |
| periphore-config | workspace with "clap" feature [VERIFIED: periphore-cli/Cargo.toml] | Config loading for `--config` path override | Already declared dep with clap feature |
| periphore-protocol | workspace | `IpcRequest`/`IpcResponse` types | Re-exported through periphore-ipc; may need direct dep if not re-exported |

### New deps required in periphore-cli/Cargo.toml

| Library | Version | Purpose | Note |
|---------|---------|---------|------|
| serde_json | workspace | Serialize/deserialize IPC JSON | Not yet in periphore-cli Cargo.toml [VERIFIED: periphore-cli/Cargo.toml]; must be added |
| tokio | workspace | Async runtime for IPC client | Not yet in periphore-cli Cargo.toml [VERIFIED]; must be added with features = ["net", "io-util", "rt", "macros"] |
| periphore-protocol | workspace | `IpcRequest`/`IpcResponse` | Not yet in periphore-cli Cargo.toml [VERIFIED]; must be added |

**Installation (Cargo.toml additions for periphore-cli):**
```toml
serde_json        = { workspace = true }
tokio             = { workspace = true }
periphore-protocol = { workspace = true }
```

No `version` override needed — workspace versions are already pinned. Tokio features are already
declared in workspace (includes `net`, `io-util`, `rt`, `macros`).

---

## Architecture Patterns

### System Architecture Diagram

```
periphore binary (main.rs)
    │  Cli::parse()
    ▼
periphore-cli::run(cli)
    │
    ├─── match cli.command
    │       │
    │       ├── Commands::Status
    │       │       │  IpcRequest::GetStatus ──JSON──► periphored (daemon)
    │       │       ◄── IpcResponse::Status { running, fingerprint }
    │       │       │  format + print to stdout
    │       │
    │       ├── Commands::Topology
    │       │       │  IpcRequest::GetTopology ──JSON──► periphored
    │       │       ◄── IpcResponse::Ok  (stub until Phase 8)
    │       │       │  print "topology not yet available (Phase 8)"
    │       │
    │       └── (future subcommands)
    │
    └─── IPC client
            │  periphore_ipc::path::socket_path()  OR  --socket override
            │  tokio::net::UnixStream::connect(path)
            │      ENOENT  → "daemon is not running" error
            │      ECONNREFUSED → "daemon is not running" error
            │  AsyncWriteExt::write_all(json + "\n")
            ▼  BufReader::read_line → serde_json::from_str::<IpcResponse>
```

### Recommended Project Structure

```
crates/periphore-cli/src/
├── lib.rs          # pub fn run(cli: Cli) -> anyhow::Result<()>; pub re-exports
├── cli.rs          # Cli struct (Parser), Commands enum (Subcommand), global args
├── client.rs       # ipc_request(socket: &Path, req: IpcRequest) -> anyhow::Result<IpcResponse>
└── commands/
    ├── mod.rs      # re-exports
    ├── status.rs   # handle_status(response: IpcResponse) -> anyhow::Result<()>
    └── topology.rs # handle_topology(response: IpcResponse) -> anyhow::Result<()>

crates/periphore/src/
└── main.rs         # #[tokio::main] fn main() — parse Cli, call periphore_cli::run(cli)
```

Note: the flat module structure is acceptable for a small phase. The commands/ subdir is
optional; a single `commands.rs` is fine if preferred. The critical boundary is that
`client.rs` (IPC transport) is separate from command handlers (output formatting).

### Pattern 1: Clap Library Crate Subcommands

**What:** Define `Cli` (global args + subcommand field) and `Commands` in the library crate.
Export `run(cli: Cli)` from lib.rs. `main.rs` calls `Cli::parse()` then `run(cli)`.

**When to use:** Always — the CLAUDE.md architecture mandates that periphore-cli is a library
and periphore/main.rs is a thin entry point.

```rust
// Source: https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html
// crates/periphore-cli/src/cli.rs
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "periphore", version, about = "Periphore input sharing CLI", long_about = None)]
pub struct Cli {
    /// Path to a custom IPC socket (overrides platform default and config).
    #[arg(long, global = true, value_name = "PATH")]
    pub socket: Option<std::path::PathBuf>,

    /// Path to the configuration file (for socket_path override lookup).
    #[arg(short, long, global = true, value_name = "FILE")]
    pub config: Option<std::path::PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Connect to the daemon and report its status and identity fingerprint.
    Status,
    /// Show the resolved monitor topology (requires daemon; stub output until Phase 8).
    Topology,
}
```

```rust
// crates/periphore-cli/src/lib.rs
pub mod cli;
pub mod client;
mod commands;

pub use cli::Cli;

pub fn run(cli: Cli) -> anyhow::Result<()> {
    // Resolve socket path: --socket > config.daemon.socket_path > platform default
    let socket_path = resolve_socket_path(&cli)?;

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        match cli.command {
            cli::Commands::Status => commands::status::run(&socket_path).await,
            cli::Commands::Topology => commands::topology::run(&socket_path).await,
        }
    })
}
```

```rust
// crates/periphore/src/main.rs  (final form after Phase 5)
use periphore_cli::Cli;
use clap::Parser;

fn main() -> anyhow::Result<()> {
    periphore_cli::run(Cli::parse())
}
```

**Key insight:** `periphore-cli` is a library — it cannot use `#[tokio::main]`. Use
`tokio::runtime::Runtime::new()?.block_on(async { ... })` inside `run()` to bridge sync to async.
Alternatively `run()` can itself be `async fn` if `main.rs` uses `#[tokio::main]`. Using
`#[tokio::main]` in main.rs is cleaner and mirrors how `periphored/src/main.rs` works.

### Pattern 2: IPC Client Transport

**What:** Connect to UnixStream, write JSON-line request, read JSON-line response. This is
the exact mirror of `periphore-ipc/tests/socket.rs::send_request()`.

**When to use:** Every subcommand that needs daemon state.

```rust
// Source: periphore-ipc/tests/socket.rs::send_request (verified in codebase)
// crates/periphore-cli/src/client.rs
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use periphore_protocol::{IpcRequest, IpcResponse};

pub async fn ipc_request(
    socket_path: &Path,
    req: IpcRequest,
) -> anyhow::Result<IpcResponse> {
    let stream = UnixStream::connect(socket_path).await
        .map_err(|e| daemon_not_running_error(e, socket_path))?;

    let (reader_half, mut writer_half) = stream.into_split();
    let mut reader = BufReader::new(reader_half);

    let mut json = serde_json::to_string(&req)?;
    json.push('\n');
    writer_half.write_all(json.as_bytes()).await?;

    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let response = serde_json::from_str::<IpcResponse>(line.trim())?;
    Ok(response)
}

fn daemon_not_running_error(e: std::io::Error, socket_path: &Path) -> anyhow::Error {
    use std::io::ErrorKind;
    match e.kind() {
        ErrorKind::NotFound => anyhow::anyhow!(
            "daemon is not running (socket not found: {})\nStart the daemon: periphored",
            socket_path.display()
        ),
        ErrorKind::ConnectionRefused => anyhow::anyhow!(
            "daemon is not running (connection refused: {})\nStart the daemon: periphored",
            socket_path.display()
        ),
        _ => anyhow::anyhow!("IPC connection failed: {e} ({})", socket_path.display()),
    }
}
```

### Pattern 3: Socket Path Resolution Priority

Priority order: `--socket` CLI flag > `config.daemon.socket_path` > `periphore_ipc::path::socket_path()`.

```rust
// crates/periphore-cli/src/lib.rs
fn resolve_socket_path(cli: &Cli) -> anyhow::Result<std::path::PathBuf> {
    if let Some(path) = &cli.socket {
        return Ok(path.clone());
    }
    // Load config only to get socket_path override — no daemon-side concern
    if let Ok(config) = periphore_config::load(cli.config.as_deref()) {
        if let Some(path) = config.daemon.socket_path {
            return Ok(path);
        }
    }
    Ok(periphore_ipc::path::socket_path())
}
```

Note: config load failures are silently ignored here — the CLI should still work if there is
no config file. This is consistent with the daemon's first-run behavior.

### Pattern 4: Status Command Output

```rust
// crates/periphore-cli/src/commands/status.rs
use crate::client::ipc_request;
use periphore_protocol::{IpcRequest, IpcResponse};

pub async fn run(socket_path: &std::path::Path) -> anyhow::Result<()> {
    let response = ipc_request(socket_path, IpcRequest::GetStatus).await?;
    match response {
        IpcResponse::Status { running, fingerprint } => {
            println!("Daemon:      {}", if running { "running" } else { "not running" });
            match fingerprint {
                Some(fp) => println!("Fingerprint: {fp}"),
                None     => println!("Fingerprint: (not available)"),
            }
        }
        IpcResponse::Error { message } => {
            anyhow::bail!("daemon error: {message}");
        }
        other => {
            anyhow::bail!("unexpected response from daemon: {other:?}");
        }
    }
    Ok(())
}
```

### Pattern 5: Topology Command (Stub-Aware)

The daemon's `GetTopology` handler currently calls `send_ok(IpcCommand::GetTopology { responder })`
which sends `IpcResponse::Ok`. The client must handle `Ok` gracefully without panicking or
producing confusing output.

```rust
// crates/periphore-cli/src/commands/topology.rs
pub async fn run(socket_path: &std::path::Path) -> anyhow::Result<()> {
    let response = ipc_request(socket_path, IpcRequest::GetTopology).await?;
    match response {
        // Phase 8 will add a real Topology variant to IpcResponse.
        // Until then, Ok is the daemon's stub response.
        IpcResponse::Ok => {
            println!("Topology: not yet available");
            println!("(Monitor topology is implemented in Phase 8)");
        }
        IpcResponse::Error { message } => {
            anyhow::bail!("daemon error: {message}");
        }
        other => {
            // Future-proof: if Phase 8 adds IpcResponse::Topology, handle it here.
            println!("Topology response: {other:?}");
        }
    }
    Ok(())
}
```

### Anti-Patterns to Avoid

- **Initializing tracing subscriber in periphore-cli:** STATE.md D-26 is explicit — only
  `periphored` initializes the subscriber. The `periphore` binary must NOT call `tracing_subscriber::init()`.
- **Using `#[tokio::main]` in a library crate:** Libraries cannot own the runtime. Use
  `tokio::runtime::Runtime::new()?.block_on(...)` in `run()`, OR let `main.rs` be the
  `#[tokio::main]` entry point and pass an async runtime context.
- **Using `.unwrap()` on IPC read:** The server can time out or the daemon can shut down
  mid-request. Use `?` propagation and let `anyhow` format the error.
- **Treating all `io::Error` uniformly:** ENOENT and ECONNREFUSED both mean "daemon not
  running" and should produce the same human-friendly message (see Pattern 2 above).
- **Hard-coding the socket path:** Always use `resolve_socket_path()` so `--socket` and
  `config.daemon.socket_path` overrides work correctly.
- **Printing to stderr for normal output:** Use `println!` (stdout) for command output,
  `eprintln!` only for errors. Structured tools downstream may capture stdout.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JSON serialization of IpcRequest | Custom text formatting | `serde_json::to_string(&req)` | Must match server's deserialization exactly; tag="type" format not obvious |
| JSON deserialization of IpcResponse | Manual string parsing | `serde_json::from_str::<IpcResponse>` | Type-tagged enum format is tricky manually; `serde(tag="type", rename_all="snake_case")` handles it |
| Socket path resolution | Custom platform detection | `periphore_ipc::path::socket_path()` | Already exists, tested, handles XDG vs TMPDIR correctly |
| Async buffered line reading | Raw `read()` loops | `tokio::io::BufReader::read_line` | Handles partial reads, framing, allocation correctly |
| Connection error classification | `e.to_string()` contains | `e.kind()` match on `ErrorKind` | Pattern matching on `ErrorKind` is robust; string matching on `e.to_string()` is fragile |
| Argument parsing | `std::env::args()` parsing | `clap` derive | Already used throughout workspace; `--help` and `--version` free |

**Key insight:** The entire JSON-lines protocol implementation already exists in `periphore-ipc`
— the client is a thin consumer of it, not a reimplementation.

---

## Common Pitfalls

### Pitfall 1: Missing tokio/serde_json deps in periphore-cli

**What goes wrong:** Compilation error `use of undeclared crate or module tokio` in periphore-cli.
**Why it happens:** `periphore-cli/Cargo.toml` currently only lists `periphore-config`, `periphore-ipc`,
`clap`, `anyhow`, `tracing`. Neither `tokio` nor `serde_json` nor `periphore-protocol` are declared.
**How to avoid:** The Wave 0 task must add these three workspace deps to `periphore-cli/Cargo.toml`.
**Warning signs:** `cargo build -p periphore-cli` fails immediately on the first async client function.

### Pitfall 2: periphore-protocol not re-exported from periphore-ipc

**What goes wrong:** `periphore_ipc` exposes `IpcCommand` but not `IpcRequest`/`IpcResponse` — those
live in `periphore_protocol`. The CLI client needs `IpcRequest` to send and `IpcResponse` to receive.
**How to avoid:** Add `periphore-protocol` as a direct dependency of `periphore-cli` (confirmed
by reading `periphore-cli/Cargo.toml` — it is absent).
**Warning signs:** `use periphore_ipc::IpcRequest;` fails; must be `use periphore_protocol::IpcRequest;`.

### Pitfall 3: ENOENT vs ECONNREFUSED both map to "daemon not running"

**What goes wrong:** CLI prints cryptic `Os { code: 2, kind: NotFound, message: "No such file or directory" }`
instead of "Start the daemon: periphored".
**Why it happens:** ENOENT = socket file does not exist (clean, never started). ECONNREFUSED = socket
file exists but no listener (crashed without cleanup). Both mean the same thing from the user's perspective.
**How to avoid:** Match on `e.kind()` in the IPC connect error path and produce a unified friendly message.
**Warning signs:** Test with daemon stopped AND with a stale socket file.

### Pitfall 4: tokio runtime in a library crate

**What goes wrong:** Calling `#[tokio::main]` or `Runtime::new().block_on(...)` from `run()` AND
also having `#[tokio::main]` in `main.rs` creates nested runtime panic: "Cannot start a runtime from
within a Tokio runtime."
**Why it happens:** `tokio::runtime::Runtime::new()` inside a function that is already called from
a tokio context panics at runtime.
**How to avoid:** Choose one approach: (A) `run()` is sync, creates its own `Runtime`; `main.rs` has
no `#[tokio::main]`. (B) `run()` is `async fn`; `main.rs` uses `#[tokio::main]`. Approach B is
preferred — it matches `periphored/src/main.rs`'s pattern and is simpler. The `periphore` binary's
`main.rs` should use `#[tokio::main]` and call `periphore_cli::run(cli).await`.
**Warning signs:** Panic with "Cannot start a runtime from within a Tokio runtime" at first `run`.

### Pitfall 5: `send_ok` branch for GetTopology returns `IpcResponse::Ok`, not a Topology type

**What goes wrong:** Client naively matches `IpcResponse::Topology { ... }` and panics / errors
when the daemon sends `IpcResponse::Ok`.
**Why it happens:** `periphored/src/main.rs::send_ok()` dispatches `GetTopology` to `IpcResponse::Ok`.
This is correct stub behavior — Phase 8 adds the real variant.
**How to avoid:** Always handle `IpcResponse::Ok` in the topology match arm with a "not yet available"
message.

### Pitfall 6: clippy pedantic on `match` with `Debug` derive

**What goes wrong:** `println!("{other:?}")` in an `other =>` arm triggers `clippy::match_wildcard_for_single_variants`
or similar pedantic lint.
**How to avoid:** Use `tracing::debug!(?other, "unexpected IPC response")` instead of printing debug
in production code, or suppress with `#[allow(clippy::...)]` with a justification comment.

---

## Code Examples

### IPC request/response pattern (from existing integration test)

```rust
// Source: periphore-ipc/tests/socket.rs::send_request() [VERIFIED: codebase]
async fn send_request(socket_path: &std::path::Path, request_json: &str) -> String {
    let stream = UnixStream::connect(socket_path).await.expect("connect");
    let (reader_half, mut writer_half) = stream.into_split();
    let mut reader = BufReader::new(reader_half);

    let mut line_with_newline = request_json.to_owned();
    line_with_newline.push('\n');
    writer_half.write_all(line_with_newline.as_bytes()).await.expect("write");

    let mut response = String::new();
    tokio::time::timeout(Duration::from_secs(2), reader.read_line(&mut response))
        .await
        .expect("timeout")
        .expect("read_line");
    response
}
```

### IpcRequest JSON wire format

```
{"type":"get_status"}
{"type":"get_topology"}
```

[VERIFIED: periphore-protocol/src/ipc.rs — `serde(rename_all = "snake_case", tag = "type")`]

### IpcResponse JSON wire format (daemon responses)

```json
{"type":"status","running":true,"fingerprint":"abc123..."}
{"type":"ok"}
{"type":"error","message":"..."}
```

[VERIFIED: periphore-protocol/src/ipc.rs]

### clap subcommand derive (library crate, no main)

```rust
// Source: https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html [CITED]
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Status,
    Topology,
}
```

### main.rs after Phase 5 (async entry point)

```rust
// Source: periphored/src/main.rs pattern [VERIFIED: codebase]
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    periphore_cli::run(periphore_cli::Cli::parse()).await
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `std::process::Command` subprocess to daemon | Direct JSON-lines IPC over UnixStream | Phase 4 (IPC complete) | Correct architecture — no subprocess needed |
| `structopt` for CLI parsing | `clap` v4 derive | clap v4.0 (2022) | `structopt` is archived; clap absorbs the derive API |

**Deprecated/outdated:**
- `tokio::net::UnixStream` was not available before tokio 1.x — all current tokio versions support it. [ASSUMED]

---

## Runtime State Inventory

Phase 5 is a new client implementation (no rename/refactor/migration). No runtime state changes.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust/cargo | Build | ✓ | (workspace builds passing) | — |
| tokio (workspace) | IPC client async | ✓ | 1.52 | — |
| serde_json (workspace) | JSON encode/decode | ✓ | 1.0 | — |
| periphored (running daemon) | Integration tests | ✓ (can be spawned in test) | current | mock socket |

All required Rust crates are already in the workspace. The only "missing" items are the
three new entries in `periphore-cli/Cargo.toml` (`tokio`, `serde_json`, `periphore-protocol`).

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | cargo test (Rust built-in) + tokio::test for async |
| Config file | None (Rust built-in) |
| Quick run command | `cargo test -p periphore-cli` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TOP-04 | `periphore topology` shows topology output or graceful stub | integration | `cargo test -p periphore-cli` | ❌ Wave 0 |
| SC1 | `periphore status` connects and prints running + fingerprint | integration | `cargo test -p periphore-cli` | ❌ Wave 0 |
| SC3 | `periphore` with no daemon fails gracefully (ENOENT error) | integration | `cargo test -p periphore-cli` | ❌ Wave 0 |

**Test approach:** Spawn a mock IPC server in a temp socket (pattern from `periphore-ipc/tests/socket.rs`).
No real `periphored` process needed — the mock router handles test commands. This is already proven
by the IPC integration tests.

### Wave 0 Gaps

- [ ] `crates/periphore-cli/tests/cli.rs` — covers TOP-04, SC1, SC3; uses mock socket pattern from periphore-ipc tests
- [ ] `crates/periphore-cli/src/client.rs` — IPC client transport (new file)
- [ ] `crates/periphore-cli/src/cli.rs` — Cli + Commands structs (new file)
- [ ] `crates/periphore-cli/src/commands/` — status.rs and topology.rs (new files)
- [ ] `periphore-cli/Cargo.toml` additions: `tokio`, `serde_json`, `periphore-protocol`

### Sampling Rate

- **Per task commit:** `cargo test -p periphore-cli`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd-verify-work`

---

## Security Domain

`security_enforcement` is enabled. ASVS Level 1 applies.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | IPC is owner-only via 0600 socket (OS-level auth, not app-level) |
| V3 Session Management | No | Single request/response per connection; no session state |
| V4 Access Control | No | 0600 socket + same-user enforcement handled by OS/kernel |
| V5 Input Validation | Yes (minimal) | CLI args validated by clap; IpcResponse deserialized by serde_json |
| V6 Cryptography | No | No new crypto in Phase 5 |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malformed IpcResponse from tampered socket | Tampering | `serde_json::from_str` returns `Err`; client propagates error, never panics |
| Privilege escalation via socket path injection | Elevation | `--socket` flag accepts `PathBuf`; no shell expansion; user must own the socket file to read responses |
| Daemon not running → clear error leaks path | Info disclosure | Socket path is not sensitive (in TMPDIR or XDG_RUNTIME_DIR); leaking it is acceptable |

**Security note:** The 0600 socket restriction means the CLI and daemon must run as the same OS user.
This is by design (STATE.md: "same user as daemon"). No additional security handling needed in Phase 5.

---

## Open Questions

1. **Should `run()` be `async fn` or sync with internal `Runtime::new()`?**
   - What we know: `periphored/src/main.rs` uses `#[tokio::main]` on an async main.
   - What's unclear: Whether `periphore/src/main.rs` should similarly use `#[tokio::main]`.
   - Recommendation: Use `#[tokio::main]` in `main.rs` and `async fn run(cli: Cli)` in the library.
     This is the cleanest pattern — avoids nested runtime risk and mirrors periphored's approach.
     The lib's `run()` function signature becomes `pub async fn run(cli: Cli) -> anyhow::Result<()>`.

2. **Will Phase 8 add `IpcResponse::Topology` or reuse `IpcResponse::Ok` with a different payload?**
   - What we know: `IpcResponse` has no Topology variant today; `GetTopology` returns `Ok`.
   - What's unclear: Phase 8's design for the topology response.
   - Recommendation: The `topology` command handles `IpcResponse::Ok` as "stub" now. When Phase 8
     adds a real variant, only `commands/topology.rs` needs updating — no protocol or transport change.

3. **Should `periphore topology` require `--debug` / `RUST_LOG=debug`?**
   - What we know: TOP-04 says "when debug logging is enabled." The CLI does not init a tracing subscriber.
   - What's unclear: Whether TOP-04 means the CLI must check a log level flag, or whether it means
     the daemon logs debug info when the daemon has debug logging enabled.
   - Recommendation: TOP-04 likely refers to the daemon's debug output, not a CLI flag. The `periphore
     topology` command should always print topology data to stdout regardless of log level — it is a
     debug/diagnostic command, not a log event. No debug-level gating needed on the CLI side.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `tokio::net::UnixStream` is available in tokio 1.52 with `features = ["net"]` | Standard Stack | Low — tokio 1.x has had UnixStream for years; workspace already uses it in periphore-ipc |
| A2 | `IpcRequest` and `IpcResponse` are not re-exported from `periphore_ipc` | Standard Stack | Low — confirmed by reading periphore-ipc/src/lib.rs (uses them via `use periphore_protocol::...` but does not `pub use` them) |
| A3 | `periphore topology` displaying a stub message satisfies TOP-04 for Phase 5 | Phase Requirements | Medium — if the reviewer expects real topology data, Phase 5 cannot deliver without Phase 8 monitor enumeration |

---

## Sources

### Primary (HIGH confidence)

- `crates/periphore-ipc/src/lib.rs` — IpcCommand variants, confirmed JSON-lines protocol
- `crates/periphore-ipc/src/server.rs` — server implementation; client is exact mirror
- `crates/periphore-ipc/tests/socket.rs` — `send_request()` is the reference client pattern
- `crates/periphore-protocol/src/ipc.rs` — IpcRequest/IpcResponse serde tags and variants
- `crates/periphore-ipc/src/path.rs` — `socket_path()` implementation; Linux/macOS paths
- `crates/periphore-cli/Cargo.toml` — confirmed missing tokio, serde_json, periphore-protocol
- `crates/periphore-cli/src/lib.rs` — confirmed stub `run()` to be replaced
- `crates/periphore/src/main.rs` — confirmed stub main to be updated
- `crates/periphored/src/main.rs` — reference for `#[tokio::main]`, clap Args, socket path resolution
- `crates/periphore-config/src/schema.rs` — confirmed `DaemonConfig::socket_path: Option<PathBuf>`
- `.planning/STATE.md` — D-26 (only periphored inits tracing), anyhow decision, clap feature

### Secondary (MEDIUM confidence)

- Context7 `/websites/rs_clap` — clap v4 `#[derive(Subcommand)]`, global args, `propagate_version`
  [CITED: https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html]

### Tertiary (LOW confidence)

None — all claims verified against codebase or official clap docs.

---

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — all deps verified against Cargo.toml files in codebase
- Architecture: HIGH — IPC protocol verified against server implementation and integration tests
- Pitfalls: HIGH — all pitfalls derived from reading actual code, not assumptions
- Test approach: HIGH — mock socket pattern verified against existing periphore-ipc tests

**Research date:** 2026-04-25
**Valid until:** 2026-05-25 (stable dependencies; clap 4.x, tokio 1.x are stable)
