# 05 — Development

This document covers prerequisites, build commands, the current status of the workspace (verified on 2026-08-14), contributing conventions, and a roadmap of recommended fixes.

## 1. Prerequisites

| Requirement | Version / Detail |
|-------------|------------------|
| Rust edition | 2021 (all three crates) |
| Toolchain (verified) | cargo 1.76.0, rustc 1.76.0 |
| Docker image toolchain | `rust:1.88-slim` (image build verified with `docker build` on 2026-08-17) |
| OS note | The workspace manifest is named `cargo.toml` (lowercase) — works on Windows/macOS (case-insensitive filesystems), **breaks on Linux/CI** (see §4) |

## 2. Commands

Run from the workspace root (Windows):

```bash
cargo check --workspace    # compile check, all 3 crates
cargo build --workspace    # build lib + both binaries
cargo test --workspace     # runs tests (currently 0)
cargo clippy --workspace   # lint (currently 5 warnings)
cargo run -p axum-auth     # run the axum app (localhost:2323)
cargo run -p actix-auth    # run the actix app (127.0.0.1:8080)
```

On Linux/macOS, root-level cargo commands fail because Cargo looks for `Cargo.toml` and only `cargo.toml` exists. Workaround: run per-crate with an explicit manifest path:

```bash
cargo check --manifest-path axum-auth/Cargo.toml
cargo check --manifest-path actix-auth/Cargo.toml
cargo check --manifest-path jwt-lib/Cargo.toml
```

Permanent fix: `git mv cargo.toml Cargo.toml` and commit.

## 3. Current Status

| Check | Result | Detail |
|-------|--------|--------|
| `cargo check --workspace` | PASS | 0 errors, 2 warnings |
| `cargo build --workspace` | PASS | All 3 members build (jwt-lib lib, axum-auth bin, actix-auth bin) |
| `cargo test --workspace` | PASS (vacuous) | **0 tests** — 0 unit tests per crate, 0 doc-tests. No `#[cfg(test)]` anywhere; coverage 0% |
| `cargo clippy --workspace` | PASS | 0 errors, 5 warnings (2 auto-fixable via `cargo clippy --fix`) |

### 3.1 Warnings (exact)

| # | Location | Tool | Message |
|---|----------|------|---------|
| 1 | `jwt-lib/src/lib.rs:14:42` | rustc | `chrono::TimeDelta::minutes` is deprecated; use `TimeDelta::try_minutes` instead |
| 2 | `jwt-lib/src/lib.rs:20:5` | clippy | `needless_return`: unneeded `return` statement (`return token;` → `token`). Auto-fixable |
| 3 | `axum-auth/src/middleware/auth.rs:24:39` | clippy | `single_char_pattern`: use a `char` for `split`: `str.split(" ")` → `str.split(' ')`. Auto-fixable |
| 4 | `actix-auth/src/middleware/auth.rs:20:39` | clippy | `single_char_pattern`: `str.split(" ")` → `str.split(' ')`. Auto-fixable |
| 5 | `actix-auth/src/main.rs:6:5` | rustc | `unused_must_use`: unused `Result`: `server::setup();` → `let _ = server::setup();` (or handle the error) |

### 3.2 Blockers

| Severity | Blocker | Status |
|----------|---------|--------|
| HIGH | Root manifest named `cargo.toml` (lowercase) — breaks root-level cargo on Linux/macOS/CI | Open — needs `git mv` + commit |
| HIGH | Zero tests in all 3 crates | Open — `cargo test` passes vacuously |
| MEDIUM | 5 lint/deprecation warnings (2 rustc, 3 clippy) | Open — 2 auto-fixable |
| LOW | Docker base image vs local toolchain — Docker build not executed in the original verification | CLOSED — verified with `docker build` (`rust:1.88-slim`) on 2026-08-17 |

## 4. Contributing Conventions

### 4.1 Code layout

Both binaries follow the same module structure — keep it when adding endpoints:

- `src/main.rs` — module declarations and entry point
- `src/server.rs` — route registration and server setup
- `src/controller/mod.rs` — public handlers; `src/controller/auth.rs` — auth handlers
- `src/middleware/auth.rs` — the `Auth` extractor

Shared JWT logic belongs in `jwt-lib` only; framework-specific code belongs in the respective binary crate.

### 4.2 Commit convention

The repository uses emoji-categorized conventional commits, greppable and consistent:

```
[ :emoji: ] | <verb>: <subject> (<scope>)
```

Examples from history: `[ :sparkles: ] | create: fun - get_jwt (lib::jwt)`, `[ :passport_control: ] | create: middleware - auth (app::middleware)`, `[ 👷 ] | create: action - docker-publish > axum (container)`.

### 4.3 Branch and CI expectations

- Single branch `main`; CI triggers on push to `main`, `v*.*.*` tags, and PRs against `main`.
- The CI workflow currently runs no quality gates; contributors should run `cargo check`, `cargo test`, and `cargo clippy` locally before committing.

## 5. Roadmap of Recommended Fixes

### 5.1 Repo hygiene (from build verification)

1. `git mv cargo.toml Cargo.toml` and commit — restores Linux/CI compatibility.
2. Commit `Cargo.lock` — reproducible builds (and enables `cargo build --locked` in CI).
3. Add `[workspace.dependencies]` in the root manifest — deduplicate versions across crates.
4. ~~Fix the `Dockerfile.axum` COPY regression~~ — done in `599107f` (`COPY ./jwt-lib/ /jwt-lib/`). Remaining: workspace-aware layout (`COPY . .` + root `Cargo.toml`).
5. Apply `cargo clippy --fix --workspace` for the auto-fixable findings; manually fix the `chrono::TimeDelta::minutes` deprecation (`try_minutes`) and the unhandled `Result` in `actix-auth/src/main.rs`.

### 5.2 Test coverage (highest quality gap)

Add unit tests for:

- `jwt-lib`: token well-formedness (3 parts), encode→decode round-trip preserves email, expired-token rejection, tampered-token/wrong-key rejection, missing-`exp` rejection, `alg=none` and RS256 rejection, leeway boundary (±1 s).
- `axum-auth` / `actix-auth`: `Auth` extractor paths — missing header, malformed header, invalid token, valid token; handler responses for `/get-token`, `/secret-view`, `/public-view`, `/`.
- Optional integration tests against a live server.

Testability blockers to remove first: the hardcoded secret/TTL (injectable config), the hardcoded clock (`Utc::now()` — injectable clock), and the missing `Debug`/`PartialEq` derives on `User`/`Claims` (needed for `assert_eq!`).

### 5.3 Security remediation

Follow the P0–P3 roadmap in [03-security.md](03-security.md). The first three items (Critical): move the secret to `JWT_SECRET` env var with a fail-fast startup check and ≥32-byte key; close the unauthenticated `/get-token` minting endpoint; run the container as a non-root user on a minimal runtime image.

## 6. Further Reading

- [01-architecture.md](01-architecture.md) — layout and lifecycles
- [02-api.md](02-api.md) — endpoint reference
- [03-security.md](03-security.md) — findings and remediation roadmap
- [04-deployment.md](04-deployment.md) — Docker and CI pipeline