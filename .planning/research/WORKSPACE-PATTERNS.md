# Workspace Patterns — Reference Projects

**Researched:** 2026-04-22
**Sources:** astral-sh/uv (67 crates), typst/typst (16 crates)
**Purpose:** Inform Phase 1 workspace scaffold decisions for Periphore

---

## 1. Workspace Cargo.toml Structure

**Both projects use `resolver = "2"`, `edition = "2024"`, and `[workspace.package]` for shared metadata.**

- All crates reference shared metadata fields as `{ workspace = true }` — never duplicated per-crate
- External deps are fully declared in `[workspace.dependencies]` with version + feature sets
- Internal crates are also declared in `[workspace.dependencies]` with both `path` and `version` fields
- Individual crates reference all deps as `{ workspace = true }` — no bare path/version refs inside crate Cargo.tomls
- Workspace-level `[workspace.lints.rust]` + `[workspace.lints.clippy]` from day one; all crates: `[lints] workspace = true`
- Multiple build profiles: `release` (strip + LTO), `profiling`, `fast-build`, `dev` overrides

**Key insight:** Workspace-level lints are much harder to retrofit — set them up on day one.

---

## 2. Crate Directory Layout

- **Both projects use a flat `crates/` directory at the workspace root.** No nesting.
- Main binary lives inside `crates/`, never at the workspace root
- `members = ["crates/*"]` in the workspace Cargo.toml
- `default-members = ["crates/<primary-binary>"]` so plain `cargo build` builds only the main binary

**uv:** `crates/uv/`, `crates/uv-audit/`, `crates/uv-auth/`, ... (67 crates)
**typst:** `crates/typst/`, `crates/typst-cli/`, `crates/typst-eval/`, ... (16 crates)

Typst also includes non-crate workspace members: `docs`, `tests/`, `tests/fuzz/`, `tests/wrapper/` — useful for integration test crates.

---

## 3. Crate Granularity

**uv is extremely fine-grained** (67 crates, many with a single-purpose lib.rs under 200 lines). The philosophy: split by compile-time dependency isolation — you can depend on `uv-pep440` without pulling in the HTTP client.

**typst is coarser-grained** (16 crates, each covering a meaningful subsystem). Medium-to-large crates.

**For Periphore:** 9 crates at the typst granularity level is appropriate. Each crate has a clear subsystem boundary.

---

## 4. Binary vs Library Crate Separation

**Both projects: binary crate is inside `crates/`, not at workspace root.**

- **uv:** `crates/uv/src/bin/uv.rs` is a thin entry point calling `uv::main()`. All command logic is in `src/lib.rs`. CLI arg parsing is in a separate `uv-cli` library crate.
- **typst:** `crates/typst-cli/src/main.rs` is the binary. `crates/typst/` is a pure library (facade re-exporting subcrates).

**Implication for Periphore:**
- Main daemon binary: `crates/periphored/` (or `crates/periphore/`) with thin `src/main.rs`
- CLI tool: `crates/periphore-ctl/` with `src/main.rs`
- Neither binary at the workspace root

---

## 5. Minimal Crate Skeleton

Both projects use `src/lib.rs` as the minimum viable crate file.

For thin/foundational crates, both projects disable unnecessary test infrastructure:
```toml
[lib]
doctest = false
test = false
```

Tests live inline in larger crates or in dedicated `tests/` workspace members.

---

## 6. Inter-Crate Dependency Management

**The workspace deps pattern (do this, not bare paths):**

In workspace `Cargo.toml`:
```toml
[workspace.dependencies]
periphore-protocol = { path = "crates/periphore-protocol", version = "0.1.0" }
periphore-config   = { path = "crates/periphore-config",   version = "0.1.0" }
```

In each consuming crate's `Cargo.toml`:
```toml
[dependencies]
periphore-protocol = { workspace = true }
```

Feature activation on internal deps is done at the consumer level:
```toml
periphore-config = { workspace = true, features = ["clap"] }
```

This gates `clap`-derived CLI args only in crates that need it (e.g., `periphore-ctl`), keeping `periphore-core` free of CLI dependencies.

---

## 7. Platform-Specific Code Patterns

**uv approach:** Dedicated platform crates (`uv-unix`) with `#![cfg(unix)]` at the module root; referenced via `[target.'cfg(unix)'.dependencies]` in consuming crates.

**For Periphore:** `periphore-capture` and `periphore-inject` use `#[cfg(target_os = "macos")]` / `#[cfg(target_os = "linux")]` at the module level. These crates are still single crates — no need for separate `periphore-capture-macos` vs `periphore-capture-linux`.

---

## 8. Feature Flag Patterns

- Optional capabilities gated behind features: `clap` feature on `periphore-config` for CLI arg parsing
- Platform deps via `[target.'cfg(...)'.dependencies]`, not workspace-level features
- Per-platform conditional compilation: `cfg` attributes in source, not separate crates (for Periphore's scale)

---

## Key Lessons for Periphore

1. **Declare every crate in `[workspace.dependencies]` with both `path` and `version`** — enables feature activation per consumer without bare path refs inside crates.

2. **Binary crates belong in `crates/`, not workspace root** — `crates/periphore/` (daemon) and `crates/periphore-ctl/` (CLI), each with a thin `src/main.rs`.

3. **Set up `[workspace.lints]` and `[lints] workspace = true` on every crate from day one** — much harder to retrofit into 9 crates later.

4. **Use `default-members = ["crates/periphore"]`** so `cargo build` at the root builds only the daemon without requiring `-p`.

5. **Scaffold all 9 crate stubs in Phase 1** — each needs a Cargo.toml + `src/lib.rs`. This establishes the workspace dependency graph that all later phases build on. Creating empty crates is trivial; adding new crates to an established workspace later requires updating the workspace Cargo.toml and all consumer Cargo.tomls.

---

*Source: gitingest analysis of astral-sh/uv (67 crates) and typst/typst (16 crates), 2026-04-22*
