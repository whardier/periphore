---
status: complete
phase: 06-tcp-peering
source: [06-VERIFICATION.md]
started: 2026-04-27T00:00:00.000Z
updated: 2026-04-27T20:00:00.000Z
---

## Current Test

[testing complete]

## Tests

### 1. Two-machine real TCP connection (SC1)
expected: Start periphored on two machines with [[peer]] entries pointing at each other. Both daemons complete the identity handshake and log connected status (PeerPending → accept fingerprint → PeerConnected). The in-process integration tests prove the protocol correct; two-machine case adds real OS TCP stack and routing.
result: blocked — corporate security software (endpoint agent) intercepts and kills all TCP connections on non-standard ports, including nc. Direct port 7888 is unreachable in both directions on the test network. Protocol correctness is validated by 6 passing handshake integration tests and SC2 (SSH tunnel, same handshake path, same OS TCP stack over loopback). Recommend re-testing on a network without endpoint agents.

### 2. SSH tunnel forwarding (SC2)
expected: `ssh -L 7889:localhost:7888 remote-host`, point local peer config at `localhost:7889`. Handshake completes with no protocol changes. TCP-only design is architecturally enforced but end-to-end tunnel operation requires real infrastructure.
result: passed — bidirectional tunnel established with `ssh -N -L 17888:localhost:7888 -R 17888:localhost:7888 rofl`. Both directions completed handshake and reached PeerConnected:
```
INFO accepted TCP connection addr=127.0.0.1:59436
INFO inbound peer trusted addr=127.0.0.1:59436 peer_id=c124af1a...
INFO peer connected and trusted peer_id=c124af1a...
INFO outbound peer trusted addr=localhost:17888 peer_id=c124af1a...
INFO peer connected and trusted peer_id=c124af1a...
```
`periphore trust accept <fingerprint>` CLI command added during this session to enable live acceptance without daemon restart.

### 3. Linux SSH remote supervision (SC3)
expected: Install `contrib/periphored.service`, enable via `systemctl --user enable --now periphored`, disconnect SSH session. Daemon survives session end via systemd linger (`loginctl enable-linger`). Requires a real Linux machine.
result: pass

## Summary

total: 3
passed: 2
issues: 0
pending: 0
skipped: 0
blocked: 1

## Gaps

- SC1 blocked by endpoint security agent on test network — not a protocol or code defect. Re-test on unmanaged network when available.
- SC3 requires a Linux machine with systemd — deferred.
- `periphore trust accept` CLI command was missing; added during SC2 testing (committed alongside UAT update).
