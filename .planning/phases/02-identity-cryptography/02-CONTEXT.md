# Phase 2: Identity & Cryptography — Context

**Gathered:** 2026-04-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 2 delivers the `periphore-identity` crate — the cryptographic identity substrate:

1. Ed25519 keypair generation with persistent storage (first-run auto-create)
2. SHA-256 fingerprint derivation from the public key (deterministic, cross-platform)
3. Identicon rendering via OpenSSH Drunken Bishop algorithm (17×9 grid, terminal output)
4. Word phrase generation from fingerprint using BIP39 wordlist (6 words, space-delimited)
5. `periphored` integration: serve `GetIdenticon` and `GetWordPhrase` IPC commands from first start

**Out of scope for Phase 2:**
- Trust acceptance flow and fingerprint caching (Phase 3 — SEC-05, SEC-06)
- TCP peering handshake that uses the identity (Phase 6)
- Fingerprint conflict enforcement in config (Phase 3)

</domain>

<decisions>
## Implementation Decisions

### Keypair Storage
- **D-01:** Raw 32-byte Ed25519 seed stored as a plain binary file. Compact, no parsing, zero ambiguity. Opaque but trivially copyable for backup.
- **D-02:** Storage path: `{XDG_DATA_HOME}/periphore/key`
  - Linux: `~/.local/share/periphore/key`
  - macOS: `~/Library/Application Support/periphore/key`
  - Use `directories::ProjectDirs` from the `directories` crate (already in workspace) to resolve the platform-appropriate path.
- **D-03:** File permissions: `0600` (user read/write only) — set immediately after creation on Unix.
- **D-04:** Parent directory created on first run by `IdentityStore` if it does not exist. No config option needed — the path is unconditional.

### Identicon (Visual Fingerprint — SEC-02)
- **D-05:** Rendering algorithm: **OpenSSH Drunken Bishop**, exact dimensions and character set. Grid: 17 columns × 9 rows. Start position: center (8, 4).
- **D-06:** Header line: `+--[ED25519 256]--+`, footer line: `+--[PERIPHORE]----+` (matching OpenSSH bracket style but with project branding).
- **D-07:** Input to randomart: the 32-byte SHA-256 fingerprint of the public key (same bytes used for the hex fingerprint and word phrase — single hash, multiple views).
- **D-08:** Identicon disable: `SEC-04` satisfied by a config flag (`identity.show_identicon = false`) that suppresses identicon output. Word-phrase verification remains available regardless.

### IPC Response Format
- **D-09:** `GetIdenticon` IPC response returns a struct with both fields:
  - `fingerprint_hex: String` — 64-char lowercase hex of the SHA-256 fingerprint (e.g., `"a3f92b1e..."`)
  - `identicon: String` — pre-rendered terminal string (the full randomart block, newline-terminated, ready to print)
  - Rendering stays in `periphore-identity`; CLI just prints the `identicon` field.
- **D-10:** `GetWordPhrase` IPC response returns:
  - `words: Vec<String>` — the 6 BIP39 words as a vector
  - `phrase: String` — space-joined convenience field (e.g., `"abandon ability able about above absent"`)

### Word Phrase (SEC-03)
- **D-11:** Wordlist: **BIP39 standard 2048-word list** inlined as `static BIP39_WORDS: &[&str; 2048]` in `periphore-identity`. No external crate — include the wordlist directly.
- **D-12:** Word count: **6 words**. Derived by splitting the 32-byte SHA-256 fingerprint into 6 overlapping or sequential 11-bit index windows (BIP39 encodes 11 bits per word for 2048 words).
- **D-13:** Output format: space-delimited lowercase (e.g., `"abandon ability able about above absent"`). No punctuation, no capitalization.

### Crate Architecture
- **D-14:** `periphore-identity` **owns its persistence I/O**. The crate exposes:
  ```rust
  pub struct IdentityStore {
      pub keypair: SigningKey,  // ed25519-dalek
      pub fingerprint: [u8; 32],  // SHA-256 of public key bytes
  }
  impl IdentityStore {
      pub fn load_or_create(path: &Path) -> Result<Self, IdentityError>;
      pub fn fingerprint_hex(&self) -> String;
      pub fn identicon(&self) -> String;       // pre-rendered Drunken Bishop art
      pub fn word_phrase(&self) -> Vec<String>; // 6 BIP39 words
  }
  ```
  Daemon calls `IdentityStore::load_or_create(&key_path)` — no I/O logic in `main.rs`.
- **D-15:** First-run behavior: if the key file does not exist, generate a new `SigningKey` from a CSPRNG (`rand` + `getrandom`), write the 32-byte seed to the file, then emit:
  ```
  tracing::info!("Generated new identity: {}", fingerprint_hex);
  ```
  One log line, no user prompt, no interactive step.
- **D-16:** Subsequent runs: read the 32-byte seed file, reconstruct `SigningKey::from_bytes(&seed)`. If the file is corrupt (wrong length), return an `IdentityError::CorruptKeyFile` — daemon logs and exits.
- **D-17:** Error type: `thiserror`-derived `IdentityError` enum (consistent with rest of workspace). `anyhow` used only at the daemon boundary (`periphore-cli` / `periphored`), not in library crates.

### Claude's Discretion
- Exact Drunken Bishop character set (OpenSSH uses ` .o+=*BOX@%&#/^SE` — Claude should match this exactly)
- Whether to use `rand::rngs::OsRng` directly or `getrandom` — use whichever `ed25519-dalek` recommends for CSPRNG seed
- Whether `IdentityStore::identicon()` accepts an optional width override for future flexibility — Claude decides
- BIP39 index derivation: exact bit-slicing approach (e.g., 6 × 11-bit windows from the 32-byte hash, treating the first 66 bits) — Claude decides the exact implementation as long as it is deterministic and cross-platform

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements
- `.planning/REQUIREMENTS.md` §SEC-01, §SEC-02, §SEC-03, §SEC-04 — the 4 requirements delivered in this phase
- `.planning/ROADMAP.md` §Phase 2 — success criteria (5 items)

### Stack & Library Choices
- `.planning/research/STACK.md` §Cryptography — ed25519-dalek, sha2, identicon notes, word-phrase options
- `.planning/research/PITFALLS.md` — implementation landmines to review before coding (check for any identity-relevant entries)

### Architecture
- `.planning/research/ARCHITECTURE.md` — crate structure, IPC design, channel topology
- `.planning/phases/01-workspace-protocol-foundation/01-CONTEXT.md` §Implementation Decisions — prior decisions:
  - D-07: `[lib] doctest = false test = false` on `periphore-identity` — integration tests go in `tests/` subdir
  - D-09: `periphore-identity` is the stubbed crate for Phase 2
  - D-15: Full `IpcRequest` enum including `GetIdenticon` and `GetWordPhrase` variants (already implemented in Phase 1)
  - D-17: `periphored` error type uses `anyhow`; library crates use `thiserror`

### Workspace Dependency Inventory
The following crates are already in `[workspace.dependencies]` and available for `periphore-identity/Cargo.toml`:
- `ed25519-dalek = "2.2"` — keypair generation and signing
- `sha2 = "0.10"` — SHA-256 fingerprint
- `serde = "1.0"` — already in identity Cargo.toml
- `directories = "6.0"` — XDG path resolution (used in daemon, may need adding to identity)
- `thiserror = "2.0"` — error type derivation
- `rand` — **NOT currently in workspace.dependencies** — will need to be added for CSPRNG (check ed25519-dalek's OsRng integration first)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/periphore-identity/src/lib.rs` — stub only (2 lines: module comment). Phase 2 fills this in entirely.
- `crates/periphore-identity/Cargo.toml` — already declares `ed25519-dalek`, `sha2`, `serde`. Add `thiserror`, `directories`, and possibly `rand` or `getrandom`.
- `crates/periphored/src/main.rs` — existing IPC dispatch already has `GetIdenticon` and `GetWordPhrase` arms (returning stubs). Phase 2 replaces stubs with real `IdentityStore` calls.

### Established Patterns (from Phase 1)
- Integration tests in `tests/` subdir (not inline) — enforced by `[lib] test = false` on this crate
- `thiserror`-derived error types in library crates (`periphore-ipc` used this; `periphore-identity` should match)
- `tracing::info!` / `tracing::error!` for runtime events (not `println!` or `eprintln!`)
- Workspace deps referenced as `{ workspace = true }` in crate Cargo.tomls — never bare versions

### Integration Points
- `periphored/src/main.rs` IPC dispatch: `GetIdenticon` and `GetWordPhrase` arms need to call into `IdentityStore` methods
- `periphore-ipc` already defines `IpcResponse` — the response struct for `GetIdenticon` needs to be updated in `periphore-protocol` or `periphore-ipc` to include `fingerprint_hex` and `identicon` fields (D-09)
- `periphore-config` `[identity]` section: add `show_identicon: bool = true` field for SEC-04 (identicon disable)

</code_context>

<specifics>
## Specific Ideas

- OpenSSH Drunken Bishop character table (exact): `" .o+=*BOX@%&#/^SE"` — Claude must use this exact sequence for cross-platform identicon determinism
- The identicon header `+--[ED25519 256]--+` and footer `+--[PERIPHORE]----+` format (17 chars wide interior matching the grid width)
- BIP39 words are designed so no word is a prefix of another (prefix-free code) — this property means character-by-character typing is unambiguous
- The single SHA-256 hash of the public key serves as the canonical fingerprint representation — identicon and word phrase are both derived from these same 32 bytes, not from separate hashes

</specifics>

<deferred>
## Deferred Ideas

- Fingerprint caching and trust acceptance (SEC-05, SEC-06) — Phase 3
- TCP handshake using identity for peer authentication — Phase 6
- `periphore identity` CLI subcommand for displaying fingerprint/identicon on demand — Phase 5 (CLI Tool)
- Key rotation or re-generation command — post-v1
- Identity backup/export command — post-v1

</deferred>

---

*Phase: 02-identity-cryptography*
*Context gathered: 2026-04-22*
