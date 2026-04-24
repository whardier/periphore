---
phase: 03-configuration-trust-persistence
plan: 02
status: complete
requirements: [CFG-02, CFG-03]
files_modified:
  - crates/periphore-config/src/schema.rs
  - crates/periphore-config/tests/config.rs
---

# Plan 02 Summary — Config Schema Evolution (CFG-02, CFG-03)

## What was done

**Schema changes (`schema.rs`):**
- Added `pub name: Option<String>` to `PeerConfig` after `fingerprint` — local-only peer label, not sent over wire
- Added `MonitorConfig` struct with `id`, `name`, `width`, `height` fields (all `Option`) — no `Serialize` derive per CFG-01
- Replaced empty `TopologyConfig` with a struct containing `monitors: Vec<MonitorConfig>` with `#[serde(default, rename = "monitor")]`
- Added `#[serde(rename = "peer")]` to `Config.peers` — required so `[[peer]]` TOML array-of-tables deserializes into the `peers` field (TOML key is singular "peer", Rust field is plural "peers")

**Test changes (`tests/config.rs`):**
- Replaced all 4 `todo!()` stubs with real implementations
- `test_peer_name_field`: verifies `PeerConfig.name` parses `"work-mac"` from `[[peer]]` block
- `test_peer_name_defaults_to_none`: verifies `PeerConfig.name` is `None` when absent
- `test_topology_monitor_config`: verifies two `[[topology.monitor]]` entries deserialize with correct id/name/width/height
- `test_topology_monitors_default_empty`: verifies `monitors` defaults to empty vec with no TOML entries

## Key finding

The `Config.peers` field needed `#[serde(rename = "peer")]` because TOML uses `[[peer]]` (singular array-of-tables key) while the Rust field name is `peers` (plural). Without the rename, Figment found no `peers` key in the TOML and produced an empty vec. This mirrors the same rename pattern already used by `TopologyConfig.monitors` / `[[topology.monitor]]`.

## Verification results

- `cargo test -p periphore-config --test config`: 11/11 passed (7 pre-existing + 4 new)
- `cargo build --workspace`: exit 0, no errors
- `grep -c "Serialize" crates/periphore-config/src/schema.rs`: 1 (comment only — CFG-01 intact)
