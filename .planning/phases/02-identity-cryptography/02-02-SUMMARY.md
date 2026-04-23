---
phase: 02-identity-cryptography
plan: 02
subsystem: identity
tags: [drunken-bishop, bip39, identicon, word-phrase, sec-02, sec-03, deterministic, fingerprint]

# Dependency graph
requires:
  - phase: 02-01
    provides: "IdentityStore with load_or_create(), fingerprint [u8; 32], identicon() and word_phrase() stubs"

provides:
  - "drunken_bishop() pure function: OpenSSH 17x9 grid, LSB-first bit order, header/footer per D-05/D-06/D-07"
  - "build_border() helper: 19-char border formula (13 - label.len() trailing dashes)"
  - "word_indices() pure function: 6 sequential 11-bit windows from 32-byte fingerprint"
  - "BIP39_WORDS static &[&str; 2048] from canonical trezor/python-mnemonic english.txt"
  - "IdentityStore::identicon() and word_phrase() — stubs replaced with real implementations"
  - "SEC-02 and SEC-03 tests: all 5 un-ignored and passing (9/9 total tests pass)"

affects:
  - 02-03-identity-cryptography
  - 06-tcp-peering

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Drunken Bishop: LSB-first within each byte (byte & 0x3 then byte >>= 2) — RESEARCH.md Pitfall 3"
    - "build_border formula: format!('+--[{}]{:->width$}+', label, '', width = 13 - label.len())"
    - "word_indices: 11-bit big-endian window extraction via 3-byte u32 window and >> (13 - bit_shift)"
    - "BIP39_WORDS inlined as pub(crate) static with compile-time assert!(len == 2048)"
    - "Pure function pattern: identicon() and word_phrase() are side-effect-free, deterministic"

key-files:
  created:
    - "crates/periphore-identity/src/bip39.rs — 2048-word BIP39 English wordlist as pub(crate) static with compile-time length assertion"
  modified:
    - "crates/periphore-identity/src/lib.rs — identicon() and word_phrase() stubs replaced; drunken_bishop, build_border, word_indices helpers added"
    - "crates/periphore-identity/tests/identity.rs — 5 ignored SEC-02/SEC-03 stubs replaced with full test implementations"

key-decisions:
  - "BIP39 wordlist sourced from canonical trezor/python-mnemonic english.txt (2048 words, 'abandon' to 'zoo') — inlined as static, no runtime parsing (D-11)"
  - "Compile-time assert!(BIP39_WORDS.len() == 2048) guards against accidental truncation (RESEARCH.md Pitfall 8 / T-02-08)"
  - "Drunken Bishop character table exactly ' .o+=*BOX@%&#/^SE' matching OpenSSH (D-05)"
  - "build_border formula confirmed: 13 - label.len() trailing dashes yields correct 19-char header/footer"
  - "word_indices extracts bits big-endian from fingerprint; AND mask & 0x7FF bounds index to 0-2047 preventing out-of-bounds panic (T-02-11)"

requirements-completed:
  - SEC-02
  - SEC-03

# Metrics
duration: 2min
completed: 2026-04-23
---

# Phase 2 Plan 02: Identicon and Word Phrase Summary

**OpenSSH Drunken Bishop identicon (17x9 grid, LSB-first, ED25519/PERIPHORE headers) and BIP39 word phrase (6 words from 11-bit fingerprint windows) implemented as deterministic pure functions of the SHA-256 fingerprint**

## Performance

- **Duration:** ~2 min
- **Started:** 2026-04-23T04:38:53Z
- **Completed:** 2026-04-23T04:41:42Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- `crates/periphore-identity/src/bip39.rs` written from canonical trezor/python-mnemonic english.txt: 2048 words, `pub(crate) static BIP39_WORDS: &[&str; 2048]`, compile-time `assert!(BIP39_WORDS.len() == 2048)`
- `drunken_bishop()` private function: 17×9 grid, center start (col=8, row=4), LSB-first bit extraction (RESEARCH.md Pitfall 3 / T-02-07), exact character table `" .o+=*BOX@%&#/^SE"`, S/E markers, header `+--[ED25519 256]--+`, footer `+--[PERIPHORE]----+`
- `build_border()` helper generates correct 19-char border using `format!("+--[{}]{:->width$}+", label, "", width = 13 - label.len())`
- `word_indices()` extracts 6 sequential 11-bit windows from 32-byte fingerprint using 3-byte u32 sliding window with `& 0x7FF` mask
- `IdentityStore::identicon()` and `word_phrase()` stubs fully replaced
- All 9 identity tests pass (0 ignored): 4 SEC-01 + 3 SEC-02 + 2 SEC-03
- `cargo test --workspace` exits 0: all prior Phase 1 tests unaffected

## Task Commits

1. **Task 1: BIP39 wordlist** - `26fd6e2` (feat)
2. **Task 2: identicon() + word_phrase() + tests** - `12ef3f3` (feat)

## Files Created/Modified

- `crates/periphore-identity/src/bip39.rs` — Full 2048-word BIP39 English wordlist as `pub(crate) static BIP39_WORDS: &[&str; 2048]` with compile-time assertion
- `crates/periphore-identity/src/lib.rs` — `identicon()` and `word_phrase()` stubs replaced with real implementations; private helpers `drunken_bishop`, `build_border`, `word_indices` added
- `crates/periphore-identity/tests/identity.rs` — 5 ignored SEC-02/SEC-03 stubs replaced with full test implementations

## Decisions Made

- Used canonical trezor/python-mnemonic english.txt for BIP39 wordlist (downloaded at execution time per objective note); first word "abandon", last word "zoo", 2048 words total
- `build_border` format string `{:->width$}` with fill char `-` and `width = 13 - label.len()` produces the exact header/footer strings verified in RESEARCH.md Python simulation
- `word_indices` uses the exact algorithm from RESEARCH.md §5: `>> (13 - bit_shift)` extracts the correct 11-bit window from the 3-byte u32 without off-by-one errors

## Deviations from Plan

None — plan executed exactly as written. The BIP39 canonical wordlist includes words not present in the plan's abbreviated example (e.g., "bachelor", "balcony", "bonus", "brass", "canoe") — this is expected and correct: the plan showed only a subset for illustration.

## Known Stubs

None — all stubs from plan 02-01 are fully implemented. `identicon()` and `word_phrase()` both return real values.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes introduced. All functions are pure (no I/O). Threat register items T-02-07, T-02-08, T-02-11 mitigated as designed.

## Self-Check: PASSED

| Item | Status |
|------|--------|
| `crates/periphore-identity/src/bip39.rs` | FOUND |
| `crates/periphore-identity/src/lib.rs` | FOUND |
| `crates/periphore-identity/tests/identity.rs` | FOUND |
| Commit 26fd6e2 (Task 1) | FOUND |
| Commit 12ef3f3 (Task 2) | FOUND |
| `pub(crate) static BIP39_WORDS` in bip39.rs | FOUND |
| `assert!(BIP39_WORDS.len() == 2048)` in bip39.rs | FOUND |
| `fn drunken_bishop` in lib.rs | FOUND |
| `fn word_indices` in lib.rs | FOUND |
| `fn build_border` in lib.rs | FOUND |
| 9/9 identity tests pass, 0 ignored | VERIFIED |
| `cargo test --workspace` exits 0 | VERIFIED |

---
*Phase: 02-identity-cryptography*
*Completed: 2026-04-23*
