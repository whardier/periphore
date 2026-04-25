# Phase 5: CLI Tool (periphore-cli) - Pattern Map

**Mapped:** 2026-04-25
**Files analyzed:** 9
**Analogs found:** 9 / 9

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/periphore-cli/Cargo.toml` | config | — | `crates/periphore-ipc/Cargo.toml` | exact (same dep set: tokio, serde_json, periphore-protocol) |
| `crates/periphore-cli/src/lib.rs` | library entry | request-response | `crates/periphored/src/main.rs` | role-match (dispatch + socket path resolution) |
| `crates/periphore-cli/src/cli.rs` | config/CLI | request-response | `crates/periphored/src/main.rs` (Args struct) | role-match (clap derive pattern) |
| `crates/periphore-cli/src/client.rs` | utility | request-response | `crates/periphore-ipc/tests/socket.rs` (`send_request`) | exact (JSON-lines client pattern) |
| `crates/periphore-cli/src/commands/mod.rs` | utility | — | `crates/periphore-ipc/src/lib.rs` | role-match (re-export module) |
| `crates/periphore-cli/src/commands/status.rs` | service | request-response | `crates/periphored/src/main.rs` (GetStatus arm) | role-match (IpcResponse match + format) |
| `crates/periphore-cli/src/commands/topology.rs` | service | request-response | `crates/periphored/src/main.rs` (send_ok GetTopology arm) | role-match (stub-aware IpcResponse::Ok handling) |
| `crates/periphore/src/main.rs` | entry point | request-response | `crates/periphored/src/main.rs` | exact (`#[tokio::main]`, clap Parser) |
| `crates/periphore-cli/tests/cli.rs` | test | request-response | `crates/periphore-ipc/tests/socket.rs` | exact (mock socket + tokio::test pattern) |

---

## Pattern Assignments

### `crates/periphore-cli/Cargo.toml` (config)

**Analog:** `crates/periphore-ipc/Cargo.toml`

**Three new workspace deps to add** (lines 14-19 of analog show the exact pattern):
```toml
# Analog: crates/periphore-ipc/Cargo.toml lines 14-19
[dependencies]
periphore-protocol = { workspace = true }
tokio              = { workspace = true }
serde_json         = { workspace = true }
```

**Full dep block for periphore-cli after additions** — keep existing deps, append:
```toml
# Append to existing [dependencies] block in crates/periphore-cli/Cargo.toml
serde_json         = { workspace = true }
tokio              = { workspace = true }
periphore-protocol = { workspace = true }
```

No version fields — workspace versions are already pinned. No feature overrides — workspace
tokio features already include `net`, `io-util`, `rt`, `macros`.

---

### `crates/periphore-cli/src/lib.rs` (library entry, request-response)

**Analog:** `crates/periphored/src/main.rs` (socket path resolution pattern, lines 89-96)

**Module declarations and public API:**
```rust
// Pattern: module declarations go at top of lib.rs; pub re-export Cli for main.rs
pub mod cli;
pub mod client;
mod commands;

pub use cli::Cli;

pub async fn run(cli: Cli) -> anyhow::Result<()> {
    let socket_path = resolve_socket_path(&cli)?;
    match cli.command {
        cli::Commands::Status   => commands::status::run(&socket_path).await,
        cli::Commands::Topology => commands::topology::run(&socket_path).await,
    }
}
```

**Socket path resolution** (mirrors `crates/periphored/src/main.rs` lines 89-96):
```rust
// Analog: crates/periphored/src/main.rs lines 89-96
// Daemon resolves: config.daemon.socket_path OR periphore_ipc::path::socket_path()
// CLI adds: --socket flag as highest priority
fn resolve_socket_path(cli: &Cli) -> anyhow::Result<std::path::PathBuf> {
    if let Some(path) = &cli.socket {
        return Ok(path.clone());
    }
    if let Ok(config) = periphore_config::load(cli.config.as_deref()) {
        if let Some(path) = config.daemon.socket_path {
            return Ok(path);
        }
    }
    Ok(periphore_ipc::path::socket_path())
}
```

Note: config load failures are silently ignored so `periphore` works without a config file,
consistent with the daemon's first-run behavior.

**Critical constraint from RESEARCH.md:** Do NOT call `tracing_subscriber::init()` here.
Only `periphored/src/main.rs` initializes the subscriber (D-26 in STATE.md).

---

### `crates/periphore-cli/src/cli.rs` (config/CLI, request-response)

**Analog:** `crates/periphored/src/main.rs` (Args struct, lines 7-23)

**Clap Args pattern in analog** (lines 7-23):
```rust
// Analog: crates/periphored/src/main.rs lines 7-23
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_name = "FILE")]
    config: Option<std::path::PathBuf>,

    #[arg(short, long)]
    verbose: bool,
}
```

**New Cli struct — adds global args and subcommand:**
```rust
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

Key differences from the analog's `Args`: this is `pub struct Cli` (not `struct Args`), lives
in a library crate (no `Parser::parse()` in the file itself), and uses `#[command(subcommand)]`
with a `Commands` enum. The `global = true` on `--socket` and `--config` ensures they work
before any subcommand position.

---

### `crates/periphore-cli/src/client.rs` (utility, request-response)

**Analog:** `crates/periphore-ipc/tests/socket.rs` — `send_request()` function (lines 115-135)

**Exact reference pattern** (lines 115-135):
```rust
// Analog: crates/periphore-ipc/tests/socket.rs lines 115-135
async fn send_request(socket_path: &std::path::Path, request_json: &str) -> String {
    let stream = UnixStream::connect(socket_path)
        .await
        .expect("should connect to test server");
    let (reader_half, mut writer_half) = stream.into_split();
    let mut reader = BufReader::new(reader_half);

    let mut line_with_newline = request_json.to_owned();
    line_with_newline.push('\n');
    writer_half
        .write_all(line_with_newline.as_bytes())
        .await
        .expect("write should succeed");

    let mut response = String::new();
    tokio::time::timeout(Duration::from_secs(2), reader.read_line(&mut response))
        .await
        .expect("response timeout")
        .expect("read_line error");
    response
}
```

**Production version — typed, error-propagating:**
```rust
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

**Import pattern** (mirrors server analog, `crates/periphore-ipc/src/server.rs` lines 1-8):
```rust
// Analog: crates/periphore-ipc/src/server.rs lines 1-8
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use periphore_protocol::{IpcRequest, IpcResponse};
```

**Key difference from test analog:** use `?` propagation and `anyhow::Result`, not `.expect()`.
Use `e.kind()` match on `ErrorKind` for ENOENT/ECONNREFUSED — not `e.to_string()` string matching.

---

### `crates/periphore-cli/src/commands/mod.rs` (utility)

**Analog:** `crates/periphore-ipc/src/lib.rs` (pub mod re-export pattern)

**Simple re-export module:**
```rust
// crates/periphore-cli/src/commands/mod.rs
pub mod status;
pub mod topology;
```

No additional logic — the `mod.rs` is purely a namespace container.

---

### `crates/periphore-cli/src/commands/status.rs` (service, request-response)

**Analog:** `crates/periphored/src/main.rs` — `GetStatus` match arm (lines 150-156)

**Server-side response construction pattern** (lines 150-156):
```rust
// Analog: crates/periphored/src/main.rs lines 150-156
Some(IpcCommand::GetStatus { responder }) => {
    tracing::debug!("IPC: GetStatus");
    let _ = responder.send(IpcResponse::Status {
        running:     true,
        fingerprint: Some(identity.fingerprint_hex()),
    });
}
```

**Client-side mirror — receive and format:**
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
            tracing::debug!(?other, "unexpected IPC response for GetStatus");
            anyhow::bail!("unexpected response from daemon");
        }
    }
    Ok(())
}
```

**Output convention:** `println!` (stdout) for command output; `eprintln!` only for fatal
errors. `tracing::debug!` instead of `println!("{other:?}")` to avoid pedantic clippy lint
on the wildcard arm.

---

### `crates/periphore-cli/src/commands/topology.rs` (service, request-response)

**Analog:** `crates/periphored/src/main.rs` — `send_ok` for `GetTopology` (lines 339-342)

**Server stub pattern** (lines 339-342):
```rust
// Analog: crates/periphored/src/main.rs lines 339-342
IpcCommand::GetTopology { responder } => {
    let _ = responder.send(IpcResponse::Ok);
}
```

**Client-side — handle the Ok stub gracefully:**
```rust
// crates/periphore-cli/src/commands/topology.rs
use crate::client::ipc_request;
use periphore_protocol::{IpcRequest, IpcResponse};

pub async fn run(socket_path: &std::path::Path) -> anyhow::Result<()> {
    let response = ipc_request(socket_path, IpcRequest::GetTopology).await?;
    match response {
        // Phase 8 will add a real Topology variant to IpcResponse.
        // Until then, Ok is the daemon's stub response for GetTopology.
        IpcResponse::Ok => {
            println!("Topology: not yet available");
            println!("(Monitor topology is implemented in Phase 8)");
        }
        IpcResponse::Error { message } => {
            anyhow::bail!("daemon error: {message}");
        }
        other => {
            // Future-proof: Phase 8 may add IpcResponse::Topology — handle here when added.
            tracing::debug!(?other, "unexpected IPC response for GetTopology");
            anyhow::bail!("unexpected response from daemon");
        }
    }
    Ok(())
}
```

**Critical:** The `IpcResponse::Ok` arm must NOT be treated as an error. This is the daemon's
correct stub response for `GetTopology` until Phase 8.

---

### `crates/periphore/src/main.rs` (entry point, request-response)

**Analog:** `crates/periphored/src/main.rs` (lines 1-2, 37-39)

**tokio::main pattern** (lines 37-39):
```rust
// Analog: crates/periphored/src/main.rs lines 37-39
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    // ... (daemon-specific init follows)
```

**New thin entry point:**
```rust
// crates/periphore/src/main.rs (replaces current stub)
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    periphore_cli::run(periphore_cli::Cli::parse()).await
}
```

**Critical constraints:**
- Do NOT add `tracing_subscriber::init()` — only `periphored` initializes the subscriber (D-26).
- Do NOT create a `tokio::runtime::Runtime` manually — `#[tokio::main]` handles the runtime.
- `run()` is `async fn` in the library; `main.rs` awaits it. This avoids the nested-runtime
  panic described in RESEARCH.md Pitfall 4.

---

### `crates/periphore-cli/tests/cli.rs` (test, request-response)

**Analog:** `crates/periphore-ipc/tests/socket.rs` (entire file — mock server + tokio::test pattern)

**Test server spawn pattern** (lines 32-58):
```rust
// Analog: crates/periphore-ipc/tests/socket.rs lines 32-58
async fn spawn_test_server(test_name: &str) -> (
    tokio::task::JoinHandle<std::io::Result<()>>,
    tokio::task::JoinHandle<()>,
    std::path::PathBuf,
) {
    let path = temp_socket_path(test_name);
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<IpcCommand>(16);

    let router = tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            handle_test_command(cmd);
        }
    });

    let server_path = path.clone();
    let server = tokio::spawn(async move {
        periphore_ipc::serve(&server_path, cmd_tx).await
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    (server, router, path)
}
```

**Unique temp path pattern** (lines 20-25):
```rust
// Analog: crates/periphore-ipc/tests/socket.rs lines 20-25
fn temp_socket_path(test_name: &str) -> std::path::PathBuf {
    let tmp = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_owned());
    std::path::PathBuf::from(tmp)
        .join("periphore-test")
        .join(format!("{test_name}-{}.sock", std::process::id()))
}
```

**Test case structure** (lines 204-226, get_status_returns_status_response):
```rust
// Analog: crates/periphore-ipc/tests/socket.rs lines 204-226
#[tokio::test]
async fn get_status_returns_status_response() {
    let (server, router, path) = spawn_test_server("get_status").await;

    let response = send_request(&path, r#"{"type":"get_status"}"#).await;

    let json: serde_json::Value =
        serde_json::from_str(response.trim()).expect("response must be valid JSON");
    assert_eq!(json["type"], "status", "response type must be 'status': {response}");
    assert_eq!(json["running"], true, "running must be true: {response}");

    server.abort();
    router.abort();
    let _ = std::fs::remove_file(&path);
}
```

**For CLI integration tests:** Instead of calling `send_request()` directly, tests should call
`crate::client::ipc_request()` (or invoke the CLI command handlers) so the typed path is tested.
The mock server setup is identical to the analog — reuse `spawn_test_server` + `handle_test_command`.

**Test teardown pattern:** Always `server.abort(); router.abort(); std::fs::remove_file(&path)` at
end of each test. No shared state between tests (each test gets a unique socket path).

---

## Shared Patterns

### IpcRequest/IpcResponse serde format

**Source:** `crates/periphore-protocol/src/ipc.rs` (lines 1-69)

**Apply to:** `client.rs`, `commands/status.rs`, `commands/topology.rs`, all future command files

```rust
// crates/periphore-protocol/src/ipc.rs lines 7-10, 42-43
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum IpcRequest { /* ... */ }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum IpcResponse { /* ... */ }
```

Wire format: `{"type":"get_status"}` request, `{"type":"status","running":true,"fingerprint":"..."}` response.
Always serialize with `serde_json::to_string(&req)` — never hand-roll JSON. Always deserialize
with `serde_json::from_str::<IpcResponse>(line.trim())`.

### JSON-lines framing

**Source:** `crates/periphore-ipc/src/server.rs` (lines 74-76, 109-119)

**Apply to:** `client.rs` write path, test `send_request` helper

```rust
// Analog: crates/periphore-ipc/src/server.rs lines 109-119
// Write: append '\n' to JSON, then write_all
let mut json = serde_json::to_string(&response)...;
json.push('\n');
writer_half.write_all(json.as_bytes()).await?;

// Read: BufReader::read_line into a String, then trim before deserializing
let mut line = String::new();
reader.read_line(&mut line).await?;
serde_json::from_str::<IpcResponse>(line.trim())?
```

### Socket path resolution

**Source:** `crates/periphore-ipc/src/path.rs` (lines 13-25), `crates/periphored/src/main.rs` (lines 89-96)

**Apply to:** `lib.rs` (`resolve_socket_path`)

```rust
// Analog: crates/periphored/src/main.rs lines 89-96
let socket_path = config
    .daemon
    .socket_path
    .clone()
    .unwrap_or_else(periphore_ipc::path::socket_path);
```

CLI adds `--socket` as the highest-priority override above config lookup.

### Error propagation with anyhow

**Source:** `crates/periphored/src/main.rs` (lines 38, 60, 72)

**Apply to:** all new files in `periphore-cli`

```rust
// Analog: crates/periphored/src/main.rs lines 60, 72
let mut config = periphore_config::load(args.config.as_deref())
    .map_err(|e| anyhow::anyhow!("failed to load config: {e}"))?;

let identity = periphore_identity::IdentityStore::load_or_create(&key_path)
    .map_err(|e| anyhow::anyhow!("identity error: {e}"))?;
```

Use `?` for error propagation. Use `.map_err(|e| anyhow::anyhow!("context: {e}"))` when adding
context. Never `.unwrap()` on IPC operations.

### tracing usage (no subscriber init)

**Source:** `crates/periphore-ipc/src/server.rs` (lines 48, 83, 99)

**Apply to:** all new files in `periphore-cli`

```rust
// Analog: crates/periphore-ipc/src/server.rs lines 48, 83, 99
tracing::info!("IPC socket listening at {}", socket_path.display());
tracing::warn!("IPC line too long ({} bytes); dropping connection", line.len());
tracing::warn!("IPC router channel closed; dropping client connection");
```

Use `tracing::debug!`, `tracing::warn!`, `tracing::error!` macros. Do NOT call
`tracing_subscriber::fmt().init()` or any subscriber initialization in `periphore-cli` crate.

---

## No Analog Found

All files have close analogs in the codebase. No entries.

---

## Metadata

**Analog search scope:** `crates/periphored/src/`, `crates/periphore-ipc/src/`, `crates/periphore-ipc/tests/`, `crates/periphore-protocol/src/`, `crates/periphore-cli/src/`, `crates/periphore/src/`
**Files scanned:** 9 analog files read in full
**Pattern extraction date:** 2026-04-25
