# Build Verification Report — rust_jwt Workspace

**Date:** 2026-08-14
**Workspace root:** `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt`
**Verification method:** `cargo` commands run from workspace root via bash tool (PowerShell shell).
**Git:** repo initialized; 16 tracked files; branch status: n/a (not checked).

## 1. Toolchain Versions

| Tool | Version |
|------|---------|
| cargo | 1.76.0 (c84b36747 2024-01-18) |
| rustc | 1.76.0 (07dca489a 2024-02-04) |
| clippy | Available (bundled with rustup toolchain) |

## 2. Manifest File Status (CRITICAL FINDING)

- Root manifest is committed and tracked by git as **`cargo.toml`** (all lowercase), **not** `Cargo.toml`.
- Member crates have correctly named manifests: `jwt-lib/Cargo.toml`, `axum-auth/Cargo.toml`, `actix-auth/Cargo.toml`.
- On **Windows (case-insensitive FS)** cargo silently resolves `cargo.toml` → `Cargo.toml`, so all commands succeeded locally.
- On **Linux/macOS/CI (case-sensitive FS)** `cargo check/build/test` at the repo root will fail with `error: could not find Cargo.toml`. Any developer running cargo at root on Linux or adding a plain cargo CI step on `ubuntu-latest` is blocked.
- Workaround verified possible: `cargo --manifest-path <crate>/Cargo.toml` per crate (each member has a proper `Cargo.toml`).
- **Fix required:** rename to `Cargo.toml` and commit (`git mv cargo.toml Cargo.toml`).
- Note: existing `.github/workflows/docker-publish-axum.yml` builds via `Dockerfile.axum` (Linux, `rust:1.71-slim`), which copies only `jwt-lib/` and `axum-auth/` into the image — it does NOT reference the root manifest, so this specific workflow is not directly broken by the filename. The lowercase manifest is a **latent blocker** for root-level cargo usage on Linux (local dev, future CI steps).
- Dockerfile note (not verified in this run — no Docker test performed): image base is `rust:1.71-slim`, older than the local 1.76 toolchain; MSRV compatibility of dependency tree (axum 0.7.4, tokio 1.36, ring 0.17.8) against rustc 1.71 is **unverified**.

## 3. Per-Command Results

### 3.1 `cargo check --workspace` — ✅ SUCCESS
- `Finished dev [unoptimized + debuginfo] target(s) in 32.09s`
- Exit: 0. Compiled `jwt-lib`, `axum-auth`, `actix-auth` (+ ~80 dependencies).
- Warnings: 2 (see section 5).

### 3.2 `cargo build --workspace` — ✅ SUCCESS
- `Finished dev [unoptimized + debuginfo] target(s) in 33.80s`
- Exit: 0. All 3 workspace members built: `jwt-lib` (lib), `axum-auth` (bin), `actix-auth` (bin).
- Warnings: 2 (same as check).

### 3.3 `cargo test --workspace` — ✅ PASS (but 0 tests exist — CRITICAL)
- `Finished test [unoptimized + debuginfo] target(s) in 1.22s`
- Exit: 0 — no failures. Test inventory:

| Target | Tests | Result |
|--------|-------|--------|
| actix-auth (bin) — unit tests | 0 | ok |
| axum-auth (bin) — unit tests | 0 | ok |
| jwt-lib (lib) — unit tests | 0 | ok |
| jwt-lib — doc-tests | 0 | ok |
| **TOTAL** | **0** | **ok (vacuous pass)** |

- **Finding:** The entire workspace has **zero tests**. No `#[cfg(test)]` modules, no integration tests. `cargo test` passes vacuously. Coverage = 0%. This is the single biggest quality gap: JWT logic, both auth middlewares, and both servers are completely untested.

### 3.4 `cargo clippy --workspace` — ✅ PASS (no errors) with 5 warnings
- `Finished dev [unoptimized + debuginfo] target(s) in 1.08s`
- Exit: 0. 2 auto-fixable suggestions available (`cargo clippy --fix`).

## 4. Per-Crate Status Summary

| Crate | check | build | test | clippy | Tests | Warnings |
|-------|-------|-------|------|--------|-------|----------|
| jwt-lib | ✅ | ✅ | ✅ (0) | ✅ 2 warn | 0 | 2 |
| axum-auth | ✅ | ✅ | ✅ (0) | ✅ 1 warn | 0 | 1 |
| actix-auth | ✅ | ✅ | ✅ (0) | ✅ 2 warn | 0 | 2 |
| **Workspace** | ✅ | ✅ | ✅ (0) | ✅ 5 warn | **0** | **5 unique** |

## 5. Clippy / Compiler Findings (exact)

1. **`jwt-lib/src/lib.rs:14:42`** — rustc `deprecated`: `chrono::TimeDelta::minutes` is deprecated; use `TimeDelta::try_minutes` instead.
   ```rust
   exp: (Utc::now() + Duration::minutes(1)).timestamp(),
   ```
2. **`jwt-lib/src/lib.rs:20:5`** — `clippy::needless_return`: unneeded `return` statement (`return token;` → `token`). Auto-fixable.
3. **`axum-auth/src/middleware/auth.rs:24:39`** — `clippy::single_char_pattern`: use `char` instead of string for `split`: `str.split(" ")` → `str.split(' ')`. Auto-fixable.
4. **`actix-auth/src/middleware/auth.rs:20:39`** — `clippy::single_char_pattern`: `str.split(" ")` → `str.split(' ')`. Auto-fixable.
5. **`actix-auth/src/main.rs:6:5`** — rustc `unused_must_use`: unused `Result` that must be used: `server::setup();` → `let _ = server::setup();` (or handle the error).

## 6. Build Blockers Discovered

| # | Severity | Blocker | Impact | Status |
|---|----------|---------|--------|--------|
| 1 | **HIGH** | Root manifest named `cargo.toml` (lowercase) — tracked in git as-is | cargo at repo root fails on Linux/macOS/CI (case-sensitive FS); only works on Windows today | Open — requires `git mv` + commit |
| 2 | **HIGH** | Zero tests in all 3 crates (0/0/0 + 0 doc-tests) | No regression protection; `cargo test` passes vacuously; violates 80%+ coverage requirement | Open |
| 3 | MEDIUM | 5 lint/deprecation warnings (2 rustc deprecation/must_use, 3 clippy) | Code hygiene; future toolchain upgrades may turn deprecations into errors | Open — 2 auto-fixable via `cargo clippy --fix` |
| 4 | LOW | `Dockerfile.axum`/`Dockerfile.actix` base `rust:1.71-slim` vs local 1.76; Docker build not executed in this verification | Potential MSRV mismatch in Dockerized CI build (unverified) | Verify with docker build |

## 7. Verdict

- **Local (Windows) build: PASS** — workspace compiles, builds, and "tests pass" (vacuously).
- **Production readiness: FAIL** — portability blocker (manifest filename) + complete absence of tests.

## 8. Recommended Next Steps (not executed)

1. `git mv cargo.toml Cargo.toml` and commit — restores Linux/CI compatibility.
2. Add unit tests for `jwt-lib` (encode/decode/expiry) and middleware auth logic in both binaries.
3. Apply `cargo clippy --fix --workspace` for the 3 auto-fixable findings; manually fix the `chrono::TimeDelta::minutes` deprecation (`try_minutes`) and the unhandled `Result` in `actix-auth/src/main.rs`.
4. Validate Docker builds (`docker build -f Dockerfile.axum .`) against rust:1.71 MSRV.
