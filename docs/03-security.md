# 03 — Security

This document summarizes the security posture of the `rust_jwt` workspace, based on a manual adversarial review of `jwt-lib`, `axum-auth`, `actix-auth`, the Docker files, and the CI workflow, verified against the vendored `jsonwebtoken` 9.2.0 source and mapped to OWASP Top 10 / CWE.

**Verdict: NOT production-ready.** The review verdict was REJECT: a critical authentication bypass is achievable with trivial effort. Do not deploy or publish images until the Critical items (C1, C2) are remediated.

## 1. Findings Summary

| Severity | Count |
|----------|-------|
| Critical | 2 |
| High | 5 |
| Medium | 8 |
| Low | 4 |
| **Total** | **19** |

> Note: the source review's summary table counts 19 findings (8 Medium), while its detailed findings table enumerates 20 rows (9 Medium, M1–M9). This discrepancy originates in the source report and is preserved here: the full table below lists every finding; the summary uses the report's headline numbers.

## 2. Security Model — What IS Protected

The codebase does enforce three things correctly:

1. **Signature verification** — `jsonwebtoken::decode` always verifies the HMAC-SHA256 signature against the secret before claims are accepted. A token signed with a different key fails with `InvalidSignature`.
2. **Expiration enforcement** — `Validation::default()` requires the `exp` claim (tokens missing it are rejected with `MissingRequiredClaim("exp")`) and rejects tokens past `exp` (with a 60-second leeway).
3. **Algorithm pinning** — `Validation::default()` restricts accepted algorithms to `[HS256]`. Verified against the 9.2.0 source: `alg: none` is rejected (`InvalidAlgorithm`/`MissingSignature`) and RS256→HS256 swaps are rejected (`InvalidAlgorithm`, plus the `DecodingKey` family check). HMAC comparison uses `ring` 0.17.8 (constant-time). **Algorithm confusion is not exploitable today — but purely by library default, not by explicit configuration.**

Everything else is weak or absent: no issuer/audience/subject validation, no revocation, no rate limiting, no TLS, a hardcoded 5-byte secret, and an unauthenticated token-minting endpoint.

## 3. Findings Table

| ID | Sev | Location | Issue | Recommended Fix |
|----|-----|----------|-------|-----------------|
| C1 | Critical | `jwt-lib/src/lib.rs:16,26` | Hardcoded JWT secret `"mykey"` (CWE-798). 5-byte secret ≈ 25–30 bits entropy — trivially brute-forced offline with a valid token (seconds–minutes). Once recovered, attacker forges tokens for any email → total auth bypass on both services. The key is also compiled into the published ghcr image binary. | Read `JWT_SECRET` from env (fail-fast at startup), generate a 256-bit (≥32-byte) key, rotate the old key, purge it from git history (BFG/filter-repo), revoke/overwrite ghcr images. |
| C2 | Critical | `axum-auth/src/controller/auth.rs:11`, `actix-auth/src/controller/auth.rs:7` | Unauthenticated token minting: `POST /get-token` accepts any JSON body `{"email": "<anything>"}` and returns a validly signed token. No client authentication whatsoever — an attacker mints tokens for any identity directly and hits `/secret-view`. The auth layer is a façade. | Gate issuance behind real credentials (password/MFA/SSO/API key); identity must be server-derived, never client-supplied. At minimum: authenticate the caller, validate email format, rate-limit. |
| H1 | High | `jwt-lib/src/lib.rs:27` | No `aud`/`iss`/`sub` validation (`Validation::default()` sets `validate_aud=false`, `iss=None`, `sub=None`). Any token signed with the same key for a different audience/issuer/service is accepted. Enables replay and cross-service confusion if the key is shared. | Explicit `Validation::new(Algorithm::HS256)` + `set_audience`, `set_issuer`, `sub`; emit `iss`/`aud`/`sub`/`iat`/`jti` at signing time. |
| H2 | High | `jwt-lib/src/lib.rs:14` | Leeway (60 s) equals 100% of the TTL (60 s): effective validity is ~2 minutes, doubling the exposure window. No refresh mechanism exists. | Set `leeway` to 0–30 s (clock-skew allowance only) and raise TTL to 15–30 min with refresh tokens, or keep 1-min TTL with leeway 0. |
| H3 | High | `Dockerfile.axum:10-18` | Container runs as root with a full Rust toolchain (`rust:1.71-slim`, EOL May 2023 → updated to `1.88-slim` in `599107f`; still ships cargo/rustc/git): large attack surface, no `USER` directive (CWE-250). Any RCE in the app = root in the container. | Multi-stage to a minimal runtime (debian slim/distroless/static musl + scratch), `USER 10001`, `--read-only` rootfs, `cap_drop: [ALL]`. |
| H4 | High | `docker-compose.yaml:8-9`, `axum-auth/src/server.rs:18`, `actix-auth/src/server.rs:21` | Bearer tokens transit plain HTTP; compose publishes `2323:2323` on `0.0.0.0` with no TLS → token theft via sniffing. (The publish also cannot reach the loopback-bound process — functional bug, see L4.) | Terminate TLS at ingress or in-app (rustls); restrict publish to `127.0.0.1:2323:2323`; never serve bearer auth over plain HTTP. |
| H5 | High | `docker-compose.yaml`, `Dockerfile.axum` | No secret delivery path: no `environment:` in compose, no secret in the workflow. The deployed image embeds the hardcoded key; rotation is impossible without rebuild. | Compose `environment: JWT_SECRET: ${JWT_SECRET}` + gitignored `.env` or Docker secrets; pass `secrets.JWT_SECRET` in CI; never bake secrets into images. |
| M1 | Medium | `jwt-lib/src/lib.rs:23-34`, `jwt-lib/src/model/user.rs` | `decode::<User>` type mismatch: tokens are encoded as `Claims{email, exp}` but decoded into `User{email}`. `exp` is enforced today only because jsonwebtoken deserializes the payload twice (second time into internal `ClaimsForValidation`). Any refactor (manual parsing, different crate, `deny_unknown_fields`) silently disables expiry. Latent auth bypass. | Decode into `Claims` and pass that through the middleware; keep `User` as a domain type constructed after validation. |
| M2 | Medium | `axum-auth/src/middleware/auth.rs:32-44`, `actix-auth/src/middleware/auth.rs:28-36`, `axum-auth/src/controller/auth.rs:29-41` | Information disclosure (CWE-209): jsonwebtoken error strings (`ExpiredSignature`, `InvalidSignature`, `MissingRequiredClaim("exp")`, `InvalidToken`) are echoed in 401/400 bodies. Acts as a validation oracle. | Return a generic `{"message": "Unauthorized"}`; log details server-side; map error kinds to status codes internally. |
| M3 | Medium | `*/src/controller/auth.rs` (both apps) | No rate limiting / lockout on `/get-token` (mint abuse) or `/secret-view` (brute-force attempts, DoS). | Per-IP + per-key limits (e.g. tower-governor / actix-web-rate-limit), exponential backoff, fail-closed. |
| M4 | Medium | `jwt-lib/src/lib.rs:12-15` | No `jti`/`iat`/`nbf` claims: no revocation capability, no issued-at audit, no early-not-valid control. A stolen token cannot be invalidated before expiry. | Add `jti` (UUID), `iat`, `nbf`; server-side denylist keyed by `jti`; `validate_nbf = true`. |
| M5 | Medium | `.gitignore:8` | `Cargo.lock` is gitignored and untracked. CI builds resolve floating dependencies → non-reproducible images, supply-chain exposure. | Commit `Cargo.lock`; add `cargo audit` / `cargo deny` to CI. |
| M6 | Medium | `.dockerignore` (1 line: `**/target`) | Build context ships `.git/`, `.github/`, docs, stray files → bloated context; any secret ever present in the working tree/history ends up in image layers. | Add `.git`, `.github`, `*.md`, `.env*`, `docs/`, `**/.git*` to `.dockerignore`. |
| M7 | Medium | `.github/workflows/docker-publish-axum.yml:34` | ~~`actions/checkout@v3` unpinned and EOL (Node 16 runtime)~~ — FIXED in `445f859` (pinned `actions/checkout@11d5960a… # v4.4.0`). All 6 third-party actions now SHA-pinned — policy consistent. | Dependabot for docker + github-actions (remaining item). |
| M8 | Medium | `*/src/controller/*.rs` (both apps) | No security headers (no `Cache-Control: no-store` on token responses, no `X-Content-Type-Options`); no authorization layer — every authenticated user reaches every endpoint; 403 never occurs; 401 vs 403 semantics undefined. | Add `Cache-Control: no-store`; define authN (401) vs authZ (403) semantics; add roles/scope claims + enforcement. |
| M9 | Medium | `jwt-lib/Cargo.toml`, `axum-auth/Cargo.toml`, `actix-auth/Cargo.toml` | Outdated dependency set: jsonwebtoken 9.2.0 (latest 9.3.0), axum 0.7.4, tokio 1.36.0, chrono 0.4.35. No `cargo audit` anywhere in CI. | Update crates; enable `cargo audit` + `cargo deny` in the workflow; pin exact versions where feasible. |
| L1 | Low | `jwt-lib/src/lib.rs:16,26` | Key hygiene compounding C1: secret length/format never checked, no startup validation, no key versioning/KID. | Validate `JWT_SECRET` length ≥ 32 bytes at startup; support `kid`/key rotation table. |
| L2 | Low | `jwt-lib/src/lib.rs:27` | Algorithm confusion (alg=none / RS256→HS256) NOT exploitable today — defense relies on library defaults, not explicit configuration. | Keep it explicit: `Validation::new(Algorithm::HS256)` + negative tests asserting `alg=none` and RS256 tokens are rejected. |
| L3 | Low | `axum-auth/src/middleware/auth.rs:24`, `actix-auth/src/middleware/auth.rs:20` | Header parsing fragility: `str.split(" ")` on literal space — double spaces/tabs misparse; `"Basic abc"` is treated as a JWT and yields an opaque 401. `Bearer` scheme never verified. | Use a proper `Authorization: Bearer <token>` parser; trim whitespace; require the `Bearer` scheme. |
| L4 | Low | `Dockerfile.axum:5`, `docker-compose.yaml:8-9` | Deployment bugs with security impact: ~~`COPY ./jwt-lib/ ../`~~ (FIXED in `599107f` → `COPY ./jwt-lib/ /jwt-lib/`); compose publishes `2323:2323` on `0.0.0.0` but the app binds `localhost` inside the container → the published port cannot reach the process (broken expose while advertising external exposure). | Add root workspace manifest, `COPY . .` with proper `.dockerignore`; bind `0.0.0.0` + TLS + firewall, or keep loopback and drop the publish mapping; add healthcheck/restart/read_only/cap_drop. |

## 4. JWT-Specific Deep Dive

### 4.1 Hardcoded secret — exploitation path

1. Obtain any valid token — trivially: `POST /get-token` is unauthenticated (C2), or sniff one over plain HTTP (H4).
2. Offline-crack the 5-byte HMAC key (`mykey`, ≈25–30 bits entropy) — minutes with hashcat wordlist/GPU.
3. Forge `{"email":"<victim>","exp":<now+3600>}` signed with the recovered key → full identity impersonation on `/secret-view` in both apps.
4. Even without steps 2–3, C2 alone yields tokens for any identity directly.

The key is also embedded in the compiled binary pushed to ghcr; image pullers can extract it with `strings`.

### 4.2 Claim validation gaps

Verified `Validation::default()`: `required_spec_claims={"exp"}`, `validate_exp=true`, `leeway=60`, `validate_nbf=false`, `validate_aud=false`, `aud=None`, `iss=None`, `sub=None`.

- **exp** — enforced, but 60 s leeway on a 60 s TTL gives a 2-minute effective lifetime (H2).
- **aud / iss / sub** — never checked (H1). Once the key is fixed, this is the highest-severity latent flaw: nothing distinguishes "issued by our auth service for this API" from any other token signed with the same key.
- **nbf** — not validated; **iat / jti** — not emitted → no revocation (M4).

### 4.3 The decode-into-User design flaw (verified — currently NOT an expiry bypass)

Tokens are signed as `Claims{email, exp}` but decoded as `User{email}`. Verified against jsonwebtoken 9.2.0 `decoding.rs`: the payload is deserialized **twice** — once into `User` (exp dropped), once into the internal `ClaimsForValidation` (exp parsed and validated: presence + `exp < now - leeway` → `ExpiredSignature`).

Therefore `exp` IS enforced today. The flaw is **fragility, not bypass**: correctness depends on undocumented library internals. A switch to `decode_header` + manual parse, jsonwebtoken 10, `#[serde(deny_unknown_fields)]`, or any "simplification" silently removes expiry enforcement. Severity: Medium (M1) — the highest-risk latent defect in the library.

### 4.4 Algorithm confusion resistance

`Validation::default()` = `Validation::new(Algorithm::HS256)` ⇒ `algorithms = [HS256]` (verified in `validation.rs`).

- `alg=none` → not in allowlist → `InvalidAlgorithm` (plus missing signature → `MissingSignature`). Rejected.
- RS256→HS256 swap → header alg not in `[HS256]` → rejected; `DecodingKey` family (Hmac) checked against algorithm family. Rejected.
- Signature check runs before claim deserialization — no signature bypass via malformed claims.
- HMAC verification uses `ring` (0.17.8) — constant-time comparison; no timing leak in the comparison itself.

**Conclusion:** not exploitable — but purely by library default. Finding L2 requires making it explicit and covered by negative tests, because a future "optimization" would instantly reintroduce the classic attacks.

### 4.5 Error handling / status codes

- All auth failures → 401 with the jsonwebtoken internal error string in the body (M2): attackers learn exactly why a token failed (expired vs invalid signature vs malformed) — an offline-crack confirmation oracle via the differentiated errors (leeway oracle).
- 401 vs 403: no authorization concept exists; every valid token reaches every protected route; 403 never occurs. Semantics must be defined (M8).

### 4.6 Other JWT issues

- TTL = 60 s: UX-hostile without a refresh flow; the leeway doubling (H2) makes it worse than a plain 5-minute token with 30 s leeway.
- No audience scoping: a single key with no `aud` means a token issued for axum is valid for actix and vice versa.
- Email as identity: unvalidated at mint (any string), echoed in `/secret-view` responses; no canonical subject claim.

## 5. Docker and CI Observations

### 5.1 Dockerfile.axum

| # | Issue | Sev |
|---|-------|-----|
| 1 | Runtime as root with full toolchain image (EOL `rust:1.71-slim` → `1.88-slim` since `599107f`), still no `USER` | High (H3) |
| 2 | No minimal runtime stage; binary is dynamically linked (glibc) | High (H3) |
| 3 | Base image not digest-pinned; moving tag (toolchain no longer EOL — `1.88-slim` since `599107f`) | Medium |
| 4 | ~~`COPY ./jwt-lib/ ../`~~ — FIXED (`599107f` → `COPY ./jwt-lib/ /jwt-lib/`; build verified 2026-08-17) | — |
| 5 | `cargo build --release` as root, no `cargo audit`/`cargo deny`, no `--locked` (impossible: Cargo.lock uncommitted) | Medium (M5/M9) |
| 6 | No `HEALTHCHECK` | Low |
| 7 | Secret baked into the published binary | Critical (C1) |

### 5.2 docker-compose.yaml

- H4: `ports: "2323:2323"` publishes on all host interfaces; app bound to container loopback → mapping is dead (L4), but the intent is external exposure with no TLS.
- H5: no `environment:` for `JWT_SECRET` → no secret delivery path.
- Missing hardening: `cap_drop`, `security_opt: no-new-privileges`, `read_only`, `tmpfs`, `restart`, resource limits, `healthcheck`. `version: "3.8"` is obsolete.

### 5.3 .dockerignore

Only `**/target`. Missing `.git/`, `.github/`, `.env*`, `*.md`, `docs/` — build context leaks git history (including the leaked secret) into image layers (M6).

### 5.4 GitHub Actions (docker-publish-axum.yml)

Good practices present (verified): all 6 third-party actions pinned to full commit SHAs; no `pull_request_target`/`workflow_run`; the only `run:` step uses env-var indirection for cosign signing; `id-token: write` scoped for sigstore.

Issues: ~~M7 (`actions/checkout@v3` unpinned, EOL)~~ — FIXED in `445f859` (pinned `actions/checkout@v4.4.0` by SHA); ~~`cosign-release: v2.1.1` stale~~ — FIXED (bumped to `v2.6.5`; v2.1.1 predates the sigstore TUF root rotation and fails with `invalid key`; cosign-installer also bumped v3.1.1 → v4.1.2 — its bootstrapped verify used cosign v2.1.1 with the same defect); no `cargo audit` or `--locked` build; no Dependabot; no `concurrency:`/`timeout-minutes`; H5 — no `JWT_SECRET` handling anywhere: the pipeline publishes the image containing the hardcoded key by design.

## 6. Prioritized Remediation Roadmap

### P0 — Do immediately (blocking; before any deploy/publish)

1. **C1** — Remove the hardcoded key: `JWT_SECRET` env var (≥32 random bytes), fail-fast startup check, purge from git history, rotate key, overwrite ghcr images.
2. **C2** — Close the unauthenticated minting endpoint: real client authentication, server-derived identity, rate limiting.
3. **H3** — Non-root runtime + minimal runtime image (distroless/slim + `USER`). Stop publishing images until items 1–2 are done.

### P1 — Within the first sprint

4. **H1** — Explicit `Validation::new(HS256)` + `iss`/`aud`/`sub` validation; emit the corresponding claims.
5. **H2** — Leeway ≤ 30 s; rework TTL policy (15–30 min + refresh).
6. **M1** — Decode into `Claims`; keep `User` as post-validation domain type; add negative tests (exp missing/expired, alg=none, RS256).
7. **M2/M8** — Generic error responses (log details server-side); `Cache-Control: no-store`; define 401/403 semantics.
8. **M3/M4** — Rate limiting; `jti`/`iat`/`nbf` claims; denylist for revocation.

### P2 — Within the first month

9. **H4/H5** — TLS termination + restricted port binding; wire `JWT_SECRET` through compose/CI; Docker hardening (`cap_drop`, `read_only`, `no-new-privileges`).
10. **M5/M9** — Commit `Cargo.lock`; add workspace root; `cargo audit`/`cargo deny` in CI; update crates.
11. **M6/L4** — Expand `.dockerignore`; ~~fix Dockerfile COPY~~ (done, `599107f`); workspace layout + healthcheck + restart policy.

### P3 — Hardening backlog

12. **M7** — ~~Pin `actions/checkout@v4` SHA~~ (done, `445f859`); Dependabot for actions and cargo; `concurrency` + `timeout-minutes`.
13. **L1/L3** — Key length/format validation + `kid` rotation; robust `Bearer` parser; negative unit tests for all JWT paths (current coverage: 0%).

## 7. QA Note — Test Coverage

- No `#[cfg(test)]` in `jwt-lib`, `axum-auth`, or `actix-auth`; no CI test job. Critical security paths (verification, validation, header parsing) have 0% automated coverage.
- **DoD block:** acceptance for any security remediation must include unit tests: expired-token rejection, tampered-signature rejection, `alg=none`/RS256 rejection, missing/malformed `exp` rejection, wrong `iss`/`aud` rejection, correct-token acceptance, and leeway boundary tests (±1 s).