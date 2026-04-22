# SDLC — Periphore

Engineering practices for Periphore. This document governs how code and planning artifacts are committed, versioned, and released. GSD agents MUST read this before authoring any commit.

---

## Branch Strategy

**Single branch: `main` only.** No feature branches, no release branches, no hotfix branches.

All work — code, planning artifacts, configuration, documentation — commits directly to `main`. The commit history is the audit trail. Conventional commits + atomic granularity replace the change isolation that branching provides in larger teams.

`commitizen-branch` (via prek) is configured to reject pushes from any branch other than `main`.

---

## Commit Conventions

Periphore uses **[Conventional Commits v1.0.0](https://www.conventionalcommits.org/)**. The `commitizen` hook (via prek) enforces format on every commit-msg stage. Non-conforming commits are rejected before they land.

### Format

```
<type>(<scope>): <description>

[body]

[footers]
```

- **Description**: imperative mood, lowercase, no trailing period, ≤72 chars
- **Body**: what changed and why — not how. Wrap at 72 chars. Skip if the subject line is self-explanatory.
- **Footers**: one per line, `Key: Value` format

### Types

| Type | Use for |
|------|---------|
| `feat` | New capability (user-visible behavior added) |
| `fix` | Defect corrected |
| `refactor` | Code restructured, no behavior change |
| `test` | Tests added or updated |
| `docs` | Documentation only (includes `.planning/` artifacts) |
| `chore` | Tooling, dependencies, build config, CI |
| `perf` | Measurable performance improvement |
| `ci` | CI/CD pipeline only |
| `build` | Build system (Cargo, cargo-xtask, etc.) |

### Scopes

| Scope | Area |
|-------|------|
| `peering` | Peer connection lifecycle, handshake protocol |
| `transport` | TCP layer, framing, connection management |
| `ipc` | Unix domain socket, local IPC server/client |
| `security` | Key management, fingerprinting, identity, trust decisions |
| `topology` | Monitor layout negotiation, edge definitions, offset resolution |
| `input` | Input capture (source), input injection (sink), event processing |
| `config` | Configuration parsing, validation, schema |
| `cli` | Command-line interface, argument parsing |
| `daemon` | Service lifecycle, process supervision, signal handling |
| `discovery` | Auto-discovery (mDNS or equivalent) |
| `planning` | `.planning/` GSD artifacts (roadmap, requirements, plans) |

Omit scope only when a change genuinely spans multiple areas.

### Breaking Changes

Append `!` to type/scope and include a `BREAKING CHANGE:` footer:

```
feat(transport)!: require TLS on all peer connections

Unencrypted TCP connections are no longer accepted. All peers must
present a valid certificate or the handshake is rejected.

BREAKING CHANGE: Peers using prior plaintext transport will fail to connect.
```

### Commit Granularity

**Commit early and often.** One logical unit per commit:

- One struct or trait implementation → one commit
- One bug fixed → one commit
- One refactor pass → one commit
- One planning artifact written or updated → one commit
- Code changes and planning changes **never share a commit**

This enables clean `git bisect`, targeted `git revert`, and meaningful `git log` for audit and debugging.

---

## Autonomous Commit Template

When GSD or any AI agent authors a commit, it **MUST** follow this template.

```
<type>(<scope>): <description>

<body: what changed and why — write for a future engineer reading git log>

[Phase: <N> | Plan: <name>]

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
Model: <model-id>
```

The `Phase:` line is required for `.planning/` commits. Omit for pure code commits.

The `Model:` footer records the AI model that authored or committed the work — **best-effort, not guaranteed complete**. In multi-agent workflows (e.g., researcher + roadmapper + synthesizer all feeding one commit), only the committing agent's model can be known with certainty. Sub-agent model attribution is a future improvement. When the model is genuinely unknown, omit the footer rather than guess.

> **Future:** A richer attribution scheme (e.g., `AI-Contributions:` block or a sidecar file) may be introduced when multi-model traceability becomes important for audit. Design that when the need is concrete.

### Rules for Autonomous Commits

1. **Atomic**: one logical change per commit — no batching planning + code, no batching multiple features
2. **Body is mandatory for non-trivial commits**: "as requested" or "as instructed" is not acceptable — explain the substance
3. **Type accuracy**: `docs` for `.planning/` artifacts, `feat`/`fix`/`refactor` for code — do not use `chore` for substantive changes
4. **Phase footer on planning commits**: include `Phase:` and `Plan:` when committing GSD artifacts
5. **Model footer when known**: include `Model: <model-id>` when the committing agent can identify itself; omit when uncertain

### Examples

**Planning artifact commit:**
```
docs(planning): initialize project configuration and roadmap

Establishes PROJECT.md, SDLC.md, config.json, REQUIREMENTS.md,
ROADMAP.md, and STATE.md for the Periphore project. Defines v1
scope as service-layer peering for scenarios 1-5, deferring
GUI/WUI and accessibility-based input capture to later phases.

Phase: 0 | Plan: new-project

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
Model: claude-sonnet-4-6
```

**Code feature commit:**
```
feat(security): implement Ed25519 keypair generation and fingerprint derivation

Generates an Ed25519 keypair on first run using the `ed25519-dalek`
crate. Derives a SHA-256 fingerprint from the public key used for
both identicon rendering and word-phrase verification. Keypair is
persisted in the platform keychain; fingerprint is cached in the
XDG state directory.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
Model: claude-sonnet-4-6
```

**Bug fix commit:**
```
fix(topology): resolve off-by-one in monitor edge offset calculation

Edge crossing coordinates were miscalculated when the target
monitor's origin was not at (0, 0). The source-relative offset
was not being added to the sink monitor's origin before injection,
causing the cursor to land at the wrong position on the sink.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
Model: claude-sonnet-4-6
```

---

## Pre-Commit Hooks (prek)

`prek.toml` in the repo root configures all hooks. Run `scripts/setup-development.sh` to install them. After installation, prek runs hooks automatically on git operations.

| Hook | Stage | Purpose |
|------|-------|---------|
| `check-added-large-files` | pre-commit | Prevent accidental binary/asset commits |
| `check-merge-conflict` | pre-commit | Block committed conflict markers |
| `check-shebang-scripts-are-executable` | pre-commit | Scripts must have execute bit |
| `detect-private-key` | pre-commit | Block accidental key material |
| `end-of-file-fixer` | pre-commit | Normalize file endings |
| `trailing-whitespace` | pre-commit | Strip trailing whitespace |
| `commitizen` | commit-msg | Enforce conventional commit format |
| `commitizen-branch` | pre-push | Enforce `main`-only push policy |

Run all hooks manually against all files:
```bash
prek run --all-files
```

Run a specific hook:
```bash
prek run --hook-stage commit-msg
```

---

## Versioning

Periphore uses **Semantic Versioning (semver)**:

| Bump | When |
|------|------|
| `MAJOR` | Breaking change to the peer protocol or IPC API |
| `MINOR` | New capability added (new scenario supported, new platform) |
| `PATCH` | Bug fix, security patch, documentation correction |

**Pre-1.0**: All releases are `0.x.y`. Breaking changes increment `MINOR`.

Version source of truth: `Cargo.toml` workspace root. Commitizen manages version bumps:

```bash
cz bump          # compute next version from conventional commit history
cz changelog     # update CHANGELOG.md from commit history
```

Tag format: `v<version>` (e.g., `v0.1.0`, `v0.2.0`)

---

## Changelog

`CHANGELOG.md` at the repo root is auto-generated from the conventional commit history:

```bash
cz changelog
```

Updated at each release. Not hand-edited — the commit history is the source of truth.

---

## Testing

- **Unit tests**: alongside source in `src/` using Rust `#[cfg(test)]` modules
- **Integration tests**: in `tests/` directory, can run against a live local socket
- **IPC layer**: testable without a live peer — loopback Unix socket, mock event sources
- **Input backends**: tested via mock event streams (no real hardware required in CI)
- **Topology negotiation**: tested with synthetic monitor definitions, deterministic layout resolution

```bash
cargo test                    # all tests
cargo test -- --nocapture     # with stdout (useful for topology debug output)
cargo test security::         # scoped to a module
```

---

## Release Process

1. Ensure all commits for the milestone are on `main` and all tests pass
2. Run `cz bump` to compute the next version from commit history and tag
3. Run `cz changelog` to regenerate `CHANGELOG.md`
4. Commit: `chore(release): bump version to vX.Y.Z`
5. Push with tags: `git push --follow-tags`

No manual version edits. `cz bump` reads `Cargo.toml` and the conventional commit log.

---

## Planning Commits (GSD)

GSD planning artifacts (`.planning/`) are committed to `main` alongside code but **never in the same commit as code**.

| Artifact | Commit when |
|----------|-------------|
| `PROJECT.md` | After initialization or at milestone boundary |
| `config.json` | After initialization or settings change |
| `REQUIREMENTS.md` | After requirements are defined or updated |
| `ROADMAP.md` + `STATE.md` | After roadmap created or revised |
| `PLAN.md` (phase N) | Immediately after the plan is created |
| Phase verification artifacts | After phase verification completes |

Use `docs(planning):` type. Include `Phase:` footer. Keep planning commits separate from code changes — one planning artifact per commit where possible.
