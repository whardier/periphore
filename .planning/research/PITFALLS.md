# Pitfalls — Periphore

**Researched:** 2026-04-22

---

## Critical (Must Address in Phase 1)

### P1: TCP Nagle's Algorithm — Latency Killer

**Problem:** Nagle's algorithm batches small TCP writes to reduce packet count. For input events (tiny, latency-sensitive messages), this creates 40–200ms delays. The Nagle + Delayed-ACK interaction is especially bad: sender waits to batch, receiver delays ACK — combined latency spikes of 40–75ms are intermittent and hard to diagnose.

**Prevention:** Set `TCP_NODELAY` on every peer socket immediately after connect/accept. Do this in the socket initialization code, not as a config option — it must always be on.

**Phase:** `periphore-net` (Phase 3) — first socket creation.

---

### P2: macOS Secure Input Mode — Silent CGEvent Tap Disable

**Problem:** When any app enables Secure Input (password fields, `sudo` prompts, Terminal with sudo, LastPass, 1Password), macOS silently disables all CGEvent taps. No error, no callback, no notification. The tap continues to exist but stops receiving events. This is the #1 source of "Periphore stopped working" reports in Synergy/Barrier.

**Prevention:** The captive window approach (Phase 4) naturally avoids this — a fullscreen window capturing input doesn't rely on CGEvent tap at all. For the eventual seamless mode: poll `CGEventTapIsEnabled()` every ~1 second; re-enable or alert via IPC when it goes false.

**Phase:** Seamless capture phase (deferred). Captive window avoids this entirely — validates the phasing decision.

---

### P3: evdev Permissions — Non-Root Access

**Problem:** `/dev/input/event*` (read) and `/dev/uinput` (write) are root-only by default. Daemon running as a user process can't capture or inject without privilege escalation.

**Prevention:** Ship udev rules that add the service user to the `input` group and grant group-write to `/dev/uinput`. Document this in the installation instructions. Never run the daemon as root.

```
# /etc/udev/rules.d/99-periphore.rules
KERNEL=="uinput", GROUP="input", MODE="0660"
SUBSYSTEM=="input", GROUP="input", MODE="0660"
```

**Phase:** `periphore-inject`/`periphore-capture` (Phase 4).

---

### P4: macOS CGEvent Tap — Code Signing Race

**Problem:** After re-signing the app binary (e.g., after an update), launching via Dock/Finder may install a tap that passes `tapCreate` non-nil but never fires. The tap was granted to the old signing identity. No error is reported.

**Prevention:** Verify `CGEventTapIsEnabled()` after install, not just check for nil. Alert via IPC if the tap is installed but not enabled. Re-request accessibility permission if signing changed (detection: compare bundle ID + signing identity from keychain).

**Phase:** Seamless capture (deferred).

---

## Moderate (Must Address Before First Usable Release)

### P5: Modifier Key Desync on Edge Crossing

**Problem:** User holds Shift on the source machine, cursor crosses to sink, then releases Shift. The Shift-up event may not reach the sink (lost in the edge transition, or source reclaims focus before the key-up fires). Sink is now stuck with Shift held — all subsequent typing is wrong.

**Prevention:** On `FocusTransfer`, send a synthetic key-up for all currently-pressed modifier keys on the source before transferring. On focus reclaim, resync modifier state via IPC or a full modifier reset sequence on the sink.

**Phase:** `periphore-core` state machine + `periphore-inject` (Phase 4).

---

### P6: Blocking in Async Context

**Problem:** Platform input APIs (especially macOS CGEvent and some evdev operations) use C callbacks and may block. Calling these from an async Tokio task blocks the executor thread, starving other tasks. Mouse events at 1000Hz will back up.

**Prevention:** Run blocking capture in `tokio::task::spawn_blocking`. The capture task is `spawn_blocking`-based; it sends events to an async channel that the router task reads. Never `.await` across a blocking C API call.

**Phase:** `periphore-capture` (Phase 4).

---

### P7: Unbounded Mouse Event Accumulation

**Problem:** Mouse move events arrive at ~1000Hz. If the network is slow or the sink's injection is slow, the channel fills with stale positions. Injecting all of them causes the cursor to "replay" old movement — laggy and wrong.

**Prevention:** Use a bounded channel for mouse events. Implement mouse-move coalescing: on channel full, drop old `MouseMove` events and keep only the latest. Do NOT coalesce button or key events. Distinguish event types in the channel design.

**Phase:** `periphore-core` router (Phase 2), finalized in Phase 4.

---

### P8: mDNS Discovery Fails Silently on Complex Networks

**Problem:** mDNS uses UDP multicast. Corporate networks, VLANs, and some WiFi configurations block multicast. The `mdns-sd` crate may silently find no peers — no error, just empty results. Users will be confused.

**Prevention:** Auto-discovery is optional, not required. Manual host configuration (Scenario 2) must always work. Surface discovery errors via IPC — report "no peers found after N seconds" rather than just showing an empty list. Document common network causes.

**Phase:** `periphore-net` discovery module (Phase 3).

---

### P9: Topology Conflicts Between Peer Configs

**Problem:** Machine A's config says "B is to my right." Machine B's config says "A is to my right." Both try to cross to each other's right side — impossible, and the negotiation will deadlock or produce wrong behavior.

**Prevention:** During `TopologyPropose`, validate that edge mappings are geometrically consistent. Detect conflicts (both sides claiming the same directional relationship) and reject the connection with a clear explanation. Surface the conflict via IPC with both sides' stated edges so the user can resolve.

**Phase:** `periphore-core` topology negotiation (Phase 2).

---

### P10: Fingerprint Cache Corruption

**Problem:** If the `known_peers.toml` cache is hand-edited incorrectly or corrupted, the daemon fails to parse it and either refuses all connections (too strict) or ignores it and re-prompts for all peers (too loose).

**Prevention:** Parse the cache defensively. If a peer's entry is malformed, log a warning and treat that peer as unknown (re-verify), rather than failing the entire cache load. Validate fingerprint format (32 bytes, hex-encoded) at parse time.

**Phase:** `periphore-identity` (Phase 1).

---

## Domain Lessons from Synergy/Barrier

| Pain Point | Root Cause | Periphore Mitigation |
|------------|------------|----------------------|
| Reconnection requires restart | No state recovery on reconnect; server dies = clients in undefined state | Design reconnection with explicit state re-sync via `TopologyAdvertise` on every new connection |
| GUI overwrites manual config | GUI treats config as its own state, not user-owned | No GUI in v1. Config is always user-authored. Cache and config are separate files |
| TLS nobody uses | Self-signed cert distribution is painful; users disable it | SSH tunneling first-class. Identicon+word-phrase is easier than cert management |
| Clipboard "dies" | Clipboard sharing layered on top, not part of protocol design | Design clipboard as a named channel in the protocol from day one, even if not implemented |
| Modifier keys get stuck | No key-state sync on edge crossing | Explicit modifier flush on `FocusTransfer` |
| Multi-monitor silent failures | No validation at config load time | Topology conflict detection at negotiation time; debug output via `periphore-ctl` |
| Wayland incompatibility | Deep assumption of X11 input model | Captive window first; no global grab assumption in v1 |
| "Which machine is the server?" | Inherent to client-server model | P2P eliminates the concept |
