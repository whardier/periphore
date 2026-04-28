---
phase: 6
slug: 06-tcp-peering
status: verified
threats_open: 0
asvs_level: 1
created: 2026-04-27
---

# Phase 6 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Protocol type surface | `PendingPeerInfo` serialized over Unix IPC socket to CLI clients | Fingerprint hex, identicon string, word phrase (no private key material) |
| Network frame → application | Incoming TCP bytes from peer deserialized via `postcard`; malformed data must not panic | Raw network bytes → `PeerMessage` enum |
| TCP socket → handshake | Any peer on the network can initiate a TCP connection to port 7888 | `PeerMessage::Hello` (fingerprint, public key, protocol version) |
| Pending → Active | State promotion gate; `HandshakeResult::Pending` cannot reach `ActiveConn` without explicit user action | `ConnectionControl::PromoteTrusted` (user-initiated via `periphore trust accept`) |
| macOS stdin → daemon startup | Daemon must not start headlessly over SSH on macOS (CGEvent requires local graphical session) | stdin TTY check at process start |
| IPC client → AcceptFingerprint | Trust store write + network state promotion are coupled in the `AcceptFingerprint` IPC path | Fingerprint hex → trust cache file + ConnectionManager::promote_pending |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-6-01 | Tampering / DoS | `codec.rs` — `LengthDelimitedCodec` frame length | mitigate | `LengthDelimitedCodec::builder().max_frame_length(64 * 1024)` in `split_framed()` — any frame claiming >64 KB causes `Err`, never OOM. Tested in `codec_roundtrip_hello`. | closed |
| T-6-02 | Elevation of Privilege | `connection.rs` / `handshake.rs` — `HandshakeResult::Pending` state | mitigate | Unknown peers return `HandshakeResult::Pending`; no input forwarding possible until `ConnectionControl::PromoteTrusted` received via explicit `periphore trust accept`. Tested in `handshake_unknown_peer_goes_pending` and `promote_pending` integration tests. | closed |
| T-6-03 | Tampering | `handshake.rs` — protocol version check | mitigate | `if protocol_version != PROTOCOL_VERSION` → send `HelloAck { accepted: false }` → return `Err(NetError::ProtocolVersion)` → connection task terminates. Tested in `protocol_version_mismatch`. | closed |
| T-6-04 | Spoofing | `handshake.rs` — `check_peer_fingerprint` call | mitigate | `periphore_trust::check_peer_fingerprint()` called when `PeerConfig.fingerprint` is set; mismatch → send `HelloAck { accepted: false }` → return `Err(NetError::FingerprintConflict)` → logged at ERROR + connection dropped. Tested in `fingerprint_conflict`. | closed |
| T-6-05 | Denial of Service | `manager.rs` — outbound retry connector loop | mitigate | Every backoff sleep is inside `tokio::select! { _ = token.cancelled() => return Ok(()) }` — removed peers' connector tasks exit promptly via `ConnectionManager::cancel_peer()`. D-11 peer-diff in both SIGHUP and ReloadConfig IPC arms. | closed |
| T-6-06 | Denial of Service | `periphored/src/main.rs` — macOS headless launch | mitigate | `#[cfg(target_os = "macos")] { if !stdin().is_terminal() { eprintln!(...); exit(1); } }` at top of `main()` before any async setup. Linux unaffected (cfg-gated). | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-6-01 | T-6-04 (AcceptFingerprint coupling) | `AcceptFingerprint` writes to the trust store (persistent) first, then calls `promote_pending` (best-effort). If `promote_pending` fails because the peer has already reconnected as trusted (race on reconnect), the next handshake finds the fingerprint in the trust store and auto-connects. The trust store is the authority — no security gap exists; the user accepted the fingerprint and it is persisted. This is a benign race, not a vulnerability. | Plan 06-04 author | 2026-04-27 |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-04-27 | 6 | 6 | 0 | gsd-security-auditor (artifact scan — all mitigations confirmed by plan executor self-checks) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-04-27
