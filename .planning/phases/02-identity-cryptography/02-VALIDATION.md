---
phase: 2
slug: identity-cryptography
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-22
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in tests via `cargo test` |
| **Config file** | `crates/periphore-identity/Cargo.toml` (`[lib] test = false` — all tests in `tests/`) |
| **Quick run command** | `cargo test -p periphore-identity` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p periphore-identity`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd-verify-work`:** Full workspace suite must be green
- **Max feedback latency:** ~5 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 02-01-01 | 01 | 1 | SEC-01 | — | Key file written with mode 0600 (no world-readable race) | integration | `cargo test -p periphore-identity -- test_first_run_creates_key_file` | Wave 0 | ⬜ pending |
| 02-01-02 | 01 | 1 | SEC-01 | — | Load after create returns identical keypair | integration | `cargo test -p periphore-identity -- test_load_after_create_is_identical` | Wave 0 | ⬜ pending |
| 02-01-03 | 01 | 1 | SEC-01 | — | Fingerprint is deterministic: same seed → same 64-char hex | unit | `cargo test -p periphore-identity -- test_fingerprint_determinism` | Wave 0 | ⬜ pending |
| 02-01-04 | 01 | 1 | SEC-01 | — | Corrupt key file returns IdentityError::CorruptKeyFile | integration | `cargo test -p periphore-identity -- test_corrupt_key_file_error` | Wave 0 | ⬜ pending |
| 02-02-01 | 02 | 2 | SEC-02 | — | Identicon is deterministic: same fingerprint → identical output | unit | `cargo test -p periphore-identity -- test_identicon_determinism` | Wave 0 | ⬜ pending |
| 02-02-02 | 02 | 2 | SEC-02 | — | Identicon header is `+--[ED25519 256]--+`, footer is `+--[PERIPHORE]----+` | unit | `cargo test -p periphore-identity -- test_identicon_borders` | Wave 0 | ⬜ pending |
| 02-02-03 | 02 | 2 | SEC-02 | — | Identicon output is exactly 11 lines (header + 9 grid rows + footer) | unit | `cargo test -p periphore-identity -- test_identicon_line_count` | Wave 0 | ⬜ pending |
| 02-02-04 | 02 | 2 | SEC-03 | — | Word phrase is deterministic: same fingerprint → same 6 words | unit | `cargo test -p periphore-identity -- test_word_phrase_determinism` | Wave 0 | ⬜ pending |
| 02-02-05 | 02 | 2 | SEC-03 | — | Word phrase has exactly 6 words, all lowercase, all valid BIP39 | unit | `cargo test -p periphore-identity -- test_word_phrase_validity` | Wave 0 | ⬜ pending |
| 02-03-01 | 03 | 3 | SEC-04 | — | Config `identity.show_identicon` defaults to `true`, parses `false` | unit | `cargo test -p periphore-config -- test_identity_config_defaults` | Wave 0 | ⬜ pending |
| 02-03-02 | 03 | 3 | SEC-01 | — | periphored IPC GetIdenticon returns fingerprint_hex + identicon fields | integration | `cargo test -p periphore-ipc -- test_get_identicon_ipc` | Wave 0 | ⬜ pending |
| 02-03-03 | 03 | 3 | SEC-03 | — | periphored IPC GetWordPhrase returns words Vec + phrase String | integration | `cargo test -p periphore-ipc -- test_get_word_phrase_ipc` | Wave 0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/periphore-identity/tests/identity.rs` — integration and unit test stubs for all SEC-01 through SEC-03 tasks (this file does not exist yet — Wave 0 creates it)
- [ ] `crates/periphore-config/tests/` — extend with `identity` config section tests for SEC-04 (add cases to existing config test suite)
- [ ] `crates/periphore-ipc/tests/` or integration tests — GetIdenticon / GetWordPhrase IPC response shape tests

*Golden-value approach: Test seed `[0u8; 32]` is used for determinism tests. During Wave 1 implementation, compute actual expected values and hard-code them in Wave 2 test assertions.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Identicon visual matches `ssh-keygen -lv` on same key | SEC-02 | Requires side-by-side visual comparison with OpenSSH output | Generate a test keypair, run `ssh-keygen -lv` on its public key, compare 17×9 grid character-by-character |
| Key file mode is 0600 on fresh generate (macOS + Linux) | SEC-01 | File permission check requires platform test environment | `ls -la ~/.local/share/periphore/key` → `-rw-------` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
