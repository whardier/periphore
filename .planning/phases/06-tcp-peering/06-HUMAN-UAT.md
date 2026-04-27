---
status: partial
phase: 06-tcp-peering
source: [06-VERIFICATION.md]
started: 2026-04-27T00:00:00.000Z
updated: 2026-04-27T00:00:00.000Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Two-machine real TCP connection (SC1)
expected: Start periphored on two machines with [[peer]] entries pointing at each other. Both daemons complete the identity handshake and log connected status (PeerPending → accept fingerprint → PeerConnected). The in-process integration tests prove the protocol correct; two-machine case adds real OS TCP stack and routing.
result: [pending]

### 2. SSH tunnel forwarding (SC2)
expected: `ssh -L 7889:localhost:7888 remote-host`, point local peer config at `localhost:7889`. Handshake completes with no protocol changes. TCP-only design is architecturally enforced but end-to-end tunnel operation requires real infrastructure.
result: [pending]

### 3. Linux SSH remote supervision (SC3)
expected: Install `contrib/periphored.service`, enable via `systemctl --user enable --now periphored`, disconnect SSH session. Daemon survives session end via systemd linger (`loginctl enable-linger`). Requires a real Linux machine.
result: [pending]

## Summary

total: 3
passed: 0
issues: 0
pending: 3
skipped: 0
blocked: 0

## Gaps
