# Phase 2: Identity & Cryptography — Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-22
**Phase:** 02-identity-cryptography
**Areas discussed:** Keypair storage, Identicon style, Word phrase design, Crate architecture

---

## Keypair Storage

| Option | Description | Selected |
|--------|-------------|----------|
| Raw binary | 32-byte seed written directly to a file. Compact, no parsing, zero ambiguity. | ✓ |
| TOML with hex encoding | Human-readable `key = "<64 hex chars>"`. Inspectable but more surface area for errors. | |
| PEM-style | `-----BEGIN ED25519 PRIVATE KEY-----` wrapping. Familiar to SSH users but adds complexity. | |

**User's choice:** Raw binary

---

| Option | Description | Selected |
|--------|-------------|----------|
| XDG_DATA_HOME | `$HOME/.local/share/periphore/` on Linux, `~/Library/Application Support/periphore/` on macOS. Correct XDG semantics for persistent user data. Backed up by most tools. | ✓ |
| XDG_STATE_HOME | `$HOME/.local/state/periphore/` on Linux. For state that isn't portable. Less universally backed up. | |
| `~/.periphore/` flat | Simple dotdir. Not XDG-compliant but familiar (like `~/.ssh/`). | |

**User's choice:** XDG_DATA_HOME

---

## Identicon Style

| Option | Description | Selected |
|--------|-------------|----------|
| ANSI colored blocks | Unicode █/░ block characters with ANSI background colors. Visually rich but requires color terminal. | |
| SSH randomart style | ASCII art like OpenSSH key fingerprint randomart. Universally renderable, no color needed, immediately familiar. | ✓ |
| Unicode half-block grid | Dense 8×4 grid using ▀▄█░ half-block characters. No color, still visually distinctive. | |

**User's choice:** SSH randomart style

---

| Option | Description | Selected |
|--------|-------------|----------|
| Pre-rendered ANSI string | Daemon renders to complete terminal-ready string; CLI just prints it. | |
| Hex fingerprint only | IPC returns raw hex; CLI renders client-side. | |
| Both: hex + rendered | IPC returns struct with both raw fingerprint and pre-rendered display string. | ✓ |

**User's choice:** Both: hex + rendered

---

| Option | Description | Selected |
|--------|-------------|----------|
| OpenSSH Drunken Bishop, exact | 17×9 grid, same algorithm as ssh-keygen. Determinism proven. Familiar to developers. | ✓ |
| Custom variant | Different grid size or step algorithm. More complexity, no user benefit. | |

**User's choice:** OpenSSH Drunken Bishop, exact (17×9)

---

## Word Phrase Design

| Option | Description | Selected |
|--------|-------------|----------|
| BIP39 standard | 2048 words purpose-built for human verification: no homophones, unambiguous spelling. Inline as static data. | ✓ |
| EFF Large Wordlist | 7776 longer, more common English words. More memorable but less recognizable format. | |
| Custom curated list | Full control but requires curation work and phonetic distinctiveness verification. | |

**User's choice:** BIP39 standard (inlined as static `&[&str]`)

---

| Option | Description | Selected |
|--------|-------------|----------|
| 6 words | ~66 bits of entropy. Fast to read/type. Right tradeoff for LAN peer verification. | ✓ |
| 4 words | ~44 bits. Very quick but lower assurance. | |
| 8 words | ~88 bits. High assurance but tedious to type character-by-character. | |

**User's choice:** 6 words

---

## Crate Architecture

| Option | Description | Selected |
|--------|-------------|----------|
| Identity owns I/O | `IdentityStore::load_or_create(path)`. Crate handles all file I/O. Daemon stays thin (D-26). | ✓ |
| Pure logic, daemon owns I/O | Only crypto operations in identity crate. Daemon reads/writes raw bytes. | |
| Hybrid: pure core + path-aware helpers | Core pure types + separate IdentityStore wrapper in same crate. | |

**User's choice:** Identity owns I/O

---

| Option | Description | Selected |
|--------|-------------|----------|
| Auto-generate and log | Generate, save, emit `tracing::info!("Generated new identity: {fingerprint_hex}")`. Matches success criterion 1. | ✓ |
| Auto-generate silently | Generate and save with no tracing output. | |
| Fail with instructions | Refuse to start; require a CLI command to generate manually. | |

**User's choice:** Auto-generate and log

---

## Claude's Discretion

- Exact Drunken Bishop character set (OpenSSH: `" .o+=*BOX@%&#/^SE"`) — Claude matches exactly
- Whether to use `rand::rngs::OsRng` or `getrandom` for CSPRNG seed
- BIP39 index derivation: exact bit-slicing approach (6 × 11-bit windows from 32-byte hash)
- Whether `identicon()` accepts an optional width override parameter

## Deferred Ideas

- Fingerprint caching and trust acceptance — Phase 3
- TCP handshake using identity for peer authentication — Phase 6
- `periphore identity` CLI subcommand — Phase 5
- Key rotation / re-generation command — post-v1
- Identity backup/export command — post-v1
