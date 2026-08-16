# Security Review — rust_jwt Workspace (JWT Authentication)

**Reviewer:** Carla (QA Engineer, code-security skill) · **Date:** 2026-08-14
**Scope:** jwt-lib, axum-auth, actix-auth, Dockerfile.axum, docker-compose.yaml, .dockerignore, .github/workflows/docker-publish-axum.yml
**Method:** Manual adversarial review + verification against vendored `jsonwebtoken-9.2.0` source (`~/.cargo/registry/src/.../jsonwebtoken-9.2.0/src/{decoding,validation}.rs`) + OWASP Top 10 / CWE mapping.
**Verdict:** ❌ **REJECT** — Critical authentication bypass achievable with trivial effort. Do not deploy or publish images until P0 items are remediated.

---

## 1. Findings Summary

| Severity | Count |
|----------|-------|
| Critical | 2 |
| High     | 5 |
| Medium   | 8 |
| Low      | 4 |
| **Total** | **19** |

## 2. Findings Table

| ID | Sev | Location | Issue | Recommended Fix |
|----|-----|----------|-------|-----------------|
| C1 | **Critical** | `jwt-lib/src/lib.rs:16,26` | **Hardcoded JWT secret `"mykey"`** (CWE-798, OWASP A07). Committed and pushed to public `github.com/Samuel-Ricardo/rust_jwt`. 5-byte secret ≈ 25–30 bits entropy → trivially brute-forced offline with a valid token (hashcat/wordlist: seconds–minutes). Once recovered, attacker forges tokens for **any email** → total auth bypass on both services. Key is also compiled into the published ghcr image binary (extractable with `strings`). | Read from `std::env::var("JWT_SECRET")` (fail-fast at startup). Generate 256-bit random (≥32 bytes) key. **Rotate now**: the old key must be treated as compromised; purge from git history (BFG/filter-repo) and re-push; revoke/overwrite ghcr images. |
| C2 | **Critical** | `axum-auth/src/controller/auth.rs:11`, `actix-auth/src/controller/auth.rs:7` | **Unauthenticated token minting endpoint.** `POST /get-token` accepts any JSON body `{"email": "<anything>"}` and returns a validly signed token — **no client authentication whatsoever**. An attacker does not need to crack the key: they mint a token for `victim@corp.com` (or any identity) directly and hit `/secret-view`. The entire auth layer is a façade. | Token issuance must be gated by real credentials (password/MFA/SSO/API-key) and identity must be server-derived, never client-supplied as a trusted claim. At minimum for this demo: authenticate the caller, validate email format, rate-limit. |
| H1 | **High** | `jwt-lib/src/lib.rs:27` | **No `aud`/`iss`/`sub` validation.** `Validation::default()` sets `validate_aud=false`, `iss=None`, `sub=None` (verified in 9.2.0 source). Any token signed with the same key — minted for a different audience, issuer, or service — is accepted. Enables token replay/cross-service confusion if the key is ever shared. | Build `Validation::new(Algorithm::HS256)`, call `set_audience(&[...])`, `set_issuer(&[...])`, set `sub`; emit `iss`/`aud`/`sub`/`iat`/`jti` claims at signing time. |
| H2 | **High** | `jwt-lib/src/lib.rs:14` | **Leeway (60 s) = 100 % of TTL (60 s).** Effective token validity is **2 minutes** (`exp < now - leeway` → token usable until `exp + 60 s`), doubling the exposure window. Combined with no refresh mechanism, users re-auth every minute anyway. | Set `leeway = 0..=30` (clock skew allowance only), raise TTL to a sane value (15–30 min) with refresh tokens, or keep 1-min TTL with leeway 0. |
| H3 | **High** | `Dockerfile.axum:10-18` | **Container runs as root with a full Rust toolchain.** Runtime stage `FROM rust:1.71-slim` (EOL toolchain, May 2023) ships cargo/rustc/git — huge attack surface — and **no `USER` directive** (CWE-250): any RCE in the app = root in the container. | Multi-stage to a minimal runtime: build with `rust:1.71-slim` (or newer, pinned to digest), run on `debian:bookworm-slim`/distroless/static musl + `scratch`; `USER 10001`; `--read-only` rootfs; drop capabilities (`cap_drop: [ALL]`). |
| H4 | **High** | `docker-compose.yaml:8-9`, `axum-auth/src/server.rs:18`, `actix-auth/src/server.rs:21` | **Bearer tokens transit plain HTTP; service is network-exposed.** Apps bind loopback (good locally), but compose publishes `2323:2323` on `0.0.0.0`, exposing the service to the LAN/host network with **no TLS**. Token theft via sniffing; note the publish actually fails to reach the loopback-bound process (functional bug, see L4). | Terminate TLS at ingress/proxy or add HTTPS in-app (rustls); restrict publish to `127.0.0.1:2323:2323`; bind `0.0.0.0` only behind a network policy/firewall; never serve bearer auth over plain HTTP. |
| H5 | **High** | `docker-compose.yaml`, `Dockerfile.axum` | **No secret delivery path.** Even after moving the key to an env var, compose defines no `environment:` and the workflow passes no secret — so the deployed image still embeds whatever default/fallback exists. No rotation possible without rebuild. | Compose: `environment: JWT_SECRET: ${JWT_SECRET}` + `.env` (gitignored) or Docker secrets; CI: pass `secrets.JWT_SECRET` at build/run; never bake secrets into images. |
| M1 | Medium | `jwt-lib/src/lib.rs:23-34`, `jwt-lib/src/model/user.rs` | **`decode::<User>` type mismatch (design flaw, works by accident).** Verified against 9.2.0 source: `decode` deserializes payload into `User` *and* into an internal `ClaimsForValidation`, so `exp` is still enforced **today** — but only because of implicit library behavior. The returned `User` drops `exp`/all security claims; any future refactor (manual base64 decode, different crate, `#[serde(deny_unknown_fields)]` on `Claims`) silently disables expiry. Fragile coupling = latent auth bypass. | Decode into the real `Claims { email, exp, iat, jti, iss, aud }` and pass that through the middleware; keep `User` (no `exp`) only as a domain model constructed *after* validation. |
| M2 | Medium | `axum-auth/src/middleware/auth.rs:32-44`, `actix-auth/src/middleware/auth.rs:28-36`, `axum-auth/src/controller/auth.rs:29-41` | **Information disclosure via error bodies (CWE-209).** `e.to_string()` of jsonwebtoken errors is echoed in 401/400 responses: `ExpiredSignature`, `InvalidSignature`, `MissingRequiredClaim("exp")`, `InvalidToken`... Reveals validation internals and acts as an oracle (confirms signature correctness vs. expiry). | Return a generic `{"message": "Unauthorized"}` with `401`; log the detail server-side only. Map error kinds → status codes internally. |
| M3 | Medium | `*/src/controller/auth.rs` (both apps) | **No rate limiting / lockout** on `/get-token` (mint abuse, credential stuffing once auth is added) or `/secret-view` (token brute-force attempts, DoS). | `tower-governor`/`actix-web-rate-limit` per-IP+per-key limits; exponential backoff; fail-closed. |
| M4 | Medium | `jwt-lib/src/lib.rs:12-15` | **No `jti`, `iat`, `nbf` claims** → no revocation capability, no issued-at audit, no early-not-valid control. A stolen token cannot be invalidated before expiry. | Add `jti` (UUID), `iat`, `nbf`; maintain a server-side denylist keyed by `jti` for logout/compromise; `validate_nbf = true`. |
| M5 | Medium | `.gitignore:8` | **`Cargo.lock` is gitignored** (and not tracked — verified via `git ls-files`). CI `cargo build --release` therefore resolves **floating dependencies** at build time → non-reproducible images, supply-chain exposure (a compromised later version gets pulled silently). | Commit `Cargo.lock` (Rust guidance: applications must commit the lockfile); consider a root workspace `Cargo.toml`; add `cargo audit`/`cargo deny` to CI. |
| M6 | Medium | `.dockerignore` (1 line: `**/target`) | **Build context ships `.git/`, `.github/`, docs, stray files** → bloated context, and any secret ever present in working tree/history (including the leaked `mykey`) ends up in image layers. | Add `.git`, `.github`, `*.md`, `.env*`, `docs/`, `**/target`, `**/.git*` to `.dockerignore`. |
| M7 | Medium | `.github/workflows/docker-publish-axum.yml:34` | **`actions/checkout@v3` unpinned and EOL** (Node 16 runtime; v4 current). All other third-party actions are correctly SHA-pinned (cosign-installer, setup-buildx, login, metadata, build-push — verified). Inconsistent pinning policy; supply-chain risk on the checkout step. | Pin `actions/checkout@<full-SHA>` (v4); keep the SHA-pinning policy for all third-party actions; add Dependabot with `group: docker` + `group: github-actions` updates. |
| M8 | Medium | `*/src/controller/*.rs` (both apps) | **No security headers; no 403 semantics.** Token-returning responses lack `Cache-Control: no-store` (tokens cached by proxies/browsers); no `X-Content-Type-Options`; **zero authorization layer** — every authenticated user gets every endpoint, and `403 Forbidden` is never produced (401 vs 403 contract undefined). | Add `Cache-Control: no-store` on all `/get-token`/`/secret-view` responses; define authN (401) vs authZ (403) semantics; add roles/scope claims + middleware enforcement. |
| M9 | Medium | `jwt-lib/Cargo.toml`, `axum-auth/Cargo.toml`, `actix-auth/Cargo.toml` | **Outdated dependency set.** `jsonwebtoken 9.2.0` (latest 9.3.0), `axum 0.7.4`, `tokio 1.36.0`, `chrono 0.4.35` (pinned by loose semver in manifests; exact versions only in the uncommitted lock). No `cargo audit` anywhere in CI. | Update crates; enable `cargo audit` + `cargo deny` (licenses+advisories) in the workflow; pin exact versions where feasible. |
| L1 | Low | `jwt-lib/src/lib.rs:16,26` (key handling) | **Weak key hygiene compounding C1**: secret length/format never checked; no startup validation; no key versioning/KID. | Validate `JWT_SECRET` length ≥ 32 bytes at startup; support `kid`/key rotation table. |
| L2 | Low | `jwt-lib/src/lib.rs:27` | **Algorithm confusion (alg=none / RS256→HS256): NOT exploitable today** — verified: `Validation::default()` ⇒ `algorithms=[HS256]`; `verify_signature` rejects any header alg not in the list, rejects missing signature, and checks `DecodingKey` family (HMAC). Defense relies on library defaults, not explicit configuration. | Keep it explicit: `Validation::new(Algorithm::HS256)` + assert in tests that `alg=none` and `RS256` tokens are rejected. Add negative unit tests. |
| L3 | Low | `axum-auth/src/middleware/auth.rs:24`, `actix-auth/src/middleware/auth.rs:20` | **Header parsing fragility**: `str.split(" ")` on literal space — `"Bearer  x"` (double space) or tab-separated tokens break/misparse; `"Basic abc"` is treated as a JWT and yields an opaque 401. | Use a proper `Authorization: Bearer <token>` parser (or `axum-extra`/`actix-web` auth extractors); trim whitespace; require `Bearer` scheme. |
| L4 | Low | `Dockerfile.axum:5`, `docker-compose.yaml:8-9` | **Deployment/correctness bugs with security impact**: `COPY ./jwt-lib/ ../` copies into filesystem root (works only by accident of path-dep resolution; no root workspace manifest); compose `2323:2323` publishes to `0.0.0.0` but the app binds `localhost` **inside** the container → published port can't reach the process (broken expose, while still advertising external exposure). | Add root workspace `Cargo.toml`, `COPY . .` with proper `.dockerignore`; bind `0.0.0.0` + TLS + firewall, or keep loopback and drop the publish mapping; add `healthcheck`, `restart`, `read_only`, `cap_drop`. |

---

## 3. JWT-Specific Deep Dive

### 3.1 Signature verification & algorithm confusion — defense present (verified against jsonwebtoken 9.2.0 source)

`decode_jwt` → `jsonwebtoken::decode::<User>` → `verify_signature`:

```rust
// decoding.rs (9.2.0) — verified
if validation.validate_signature && !validation.algorithms.contains(&header.alg) {
    return Err(new_error(ErrorKind::InvalidAlgorithm));
}
if validation.validate_signature && !verify(signature, message.as_bytes(), key, header.alg)? {
    return Err(new_error(ErrorKind::InvalidSignature));
}
```

- `Validation::default()` = `Validation::new(Algorithm::HS256)` ⇒ `algorithms = [HS256]` (verified in `validation.rs`).
- **`alg=none`** → not in allowlist → `InvalidAlgorithm` (plus missing signature → `MissingSignature`). Rejected. ✅
- **RS256→HS256 swap** → header alg RS256 not in `[HS256]` → rejected; and `DecodingKey` family is `Hmac`, checked against algorithm family. Rejected. ✅
- Signature check runs **before** claim deserialization — no signature-bypass via malformed claims. ✅
- HMAC verification uses `ring` (0.17.8 in lock) — constant-time comparison. ✅ No timing leak in the comparison itself.

**Conclusion:** Algorithm confusion is *not* exploitable in this codebase — but purely by library default. Finding **L2** requires making this explicit and covered by negative tests, because a future "optimization" (e.g., `decode_header` + manual parse, or adding an RSA alg for "flexibility") would instantly reintroduce the classic attacks.

### 3.2 Claim validation gaps — the real weakness

Verified `Validation::default()`: `required_spec_claims={"exp"}`, `validate_exp=true`, `leeway=60`, `validate_nbf=false`, `validate_aud=false`, `aud=None`, `iss=None`, `sub=None`.

- **`exp`** — enforced (see 3.3) but with 60 s leeway on a 60 s TTL → **2-minute effective lifetime** (H2).
- **`aud` / `iss` / `sub`** — never checked (**H1**). Any token signed with the same key from any context is accepted; there is no way to distinguish "issued by our auth service for this API" from anything else. Combined with C1/C2 this is moot today, but it is the highest-severity *latent* flaw once the key is fixed.
- **`nbf`** — not validated; **`iat`/`jti`** — not emitted → no revocation (M4).

### 3.3 The `decode::<User>` design flaw — verified NOT an expiry bypass (currently)

The known concern: tokens are signed as `Claims{email, exp}` but decoded as `User{email}` (serde silently ignores the unknown `exp`). Verified against 9.2.0 `decoding.rs`:

```rust
Ok((header, claims)) => {
    let decoded_claims = DecodedJwtPartClaims::from_jwt_part_claims(claims)?;
    let claims = decoded_claims.deserialize()?;            // -> User (exp dropped)
    validate(decoded_claims.deserialize()?, validation)?;  // -> ClaimsForValidation (exp parsed)
    Ok(TokenData { header, claims })
}
```

The payload is deserialized **twice**: once into `User`, once into the internal `ClaimsForValidation` which carries `exp`/`nbf`/`sub`/`iss`/`aud`. `validate()` (verified) checks `required_spec_claims` (`exp` must be a *present, numeric* claim — else `MissingRequiredClaim`) and `exp < now - leeway` → `ExpiredSignature`.

**Therefore `exp` IS still enforced with `decode::<User>`.** The flaw is *fragility, not bypass*: correctness depends on undocumented library internals (the second deserialization). A switch to `decode_header` + manual parse, `jsonwebtoken` 10, `#[serde(deny_unknown_fields)]`, or any "simplification" silently removes expiry enforcement. Severity: **Medium (M1)** — latent, but the highest-risk latent defect in the library. Fix: decode into `Claims` (the real token schema), keep `User` as a post-validation domain type.

### 3.4 Hardcoded secret — exploitation path

1. Obtain any valid token (call the public `POST /get-token` with any email — endpoint is unauthenticated, C2 — or sniff one over plain HTTP, H4).
2. Offline-crack the 5-byte HMAC key (`mykey`): entropy ≈ 25–30 bits → minutes with hashcat wordlist/GPU.
3. Forge `{"email":"<victim>","exp":<now+3600>}` signed with recovered key → full identity impersonation on `/secret-view` (both axum & actix).
4. Even without step 2–3, C2 alone yields tokens for any identity directly.

Also: key is embedded in the binary pushed to ghcr — image pullers can extract it (C1 note).

### 3.5 Error handling / status codes

- All auth failures → `401 Unauthorized` with jsonwebtoken's internal error string in the body (**M2**): attacker learns *exactly* why a token failed (expired vs invalid signature vs malformed), and can confirm a guessed key via `ExpiredSignature` (vs `InvalidSignature`) — an offline-crack confirmation oracle once a token with a near-expiry is replayed... (leeway oracle).
- **401 vs 403**: no authorization concept exists — every valid token reaches every protected route; `403` never occurs. Semantics must be defined (M8): 401 = unauthenticated/bad token; 403 = authenticated but not permitted (roles/scope).

### 3.6 Other JWT issues

- **TTL = 60 s**: UX-hostile without refresh flow; every request re-auth window is tiny but the leeway doubling (H2) makes it *worse* security-wise than a plain 5-min token with 30 s leeway.
- **No audience-scoping of tokens**: single key, no `aud` — a token for axum is valid for actix and vice versa.
- **Email as identity**: unvalidated at mint (any string), echoed in `/secret-view` responses; no canonical subject claim.

---

## 4. Docker & CI/CD Section

### 4.1 Dockerfile.axum

| # | Issue | Sev | Detail |
|---|-------|-----|--------|
| 1 | Runtime as root, full toolchain image | **High (H3)** | `FROM rust:1.71-slim` runtime stage (EOL Rust 1.71, May 2023) — no `USER`, no `apt` cleanup concerns, ships rustc/cargo/git/cc. |
| 2 | No minimal runtime stage | High (H3) | Binary is dynamically linked (glibc); copy into `debian:bookworm-slim` or use `musl` static + `scratch`/`distroless`. |
| 3 | Base image not digest-pinned | Medium | `rust:1.71-slim` is a moving tag; CI + local builds can diverge; toolchain EOL anyway (no security patches for rustc itself). Pin `rust:1.71-slim@sha256:...` or move to a maintained version (1.8x) pinned by digest. |
| 4 | `COPY ./jwt-lib/ ../` | Low (L4) | Copies into `/`; works only via path-dep resolution quirk; breaks if a root workspace manifest is added. Use a workspace + `COPY . .` + `.dockerignore`. |
| 5 | `cargo build --release` as root, no `cargo audit`/`cargo deny`, no `--locked` | Medium (M5/M9) | `--locked` impossible today because `Cargo.lock` is not committed; CI builds float deps. |
| 6 | No `HEALTHCHECK` | Low | Operational; ties into compose `restart` policies. |
| 7 | Secret baked in binary | Critical (C1) | `mykey` is in the compiled artifact published to ghcr. |

### 4.2 docker-compose.yaml

- **H4:** `ports: "2323:2323"` publishes on all host interfaces; with the app bound to container-loopback the mapping is dead (L4) — but the *intent* is external exposure with no TLS.
- **H5:** no `environment:` for `JWT_SECRET` → no secret delivery path.
- Missing hardening: `cap_drop: [ALL]`, `security_opt: [no-new-privileges:true]`, `read_only: true`, `tmpfs`, `restart`, resource limits, `healthcheck`. `version: "3.8"` is obsolete (compose spec).

### 4.3 .dockerignore

- Only `**/target`. Missing: `.git/`, `.github/`, `.env*`, `*.md`, `docs/`. Build context leaks git history (incl. the leaked secret) into image layers (M6).

### 4.4 GitHub Actions (docker-publish-axum.yml)

**Good practices present (verified):**
- ✅ 5 of 6 third-party actions pinned to **full commit SHAs** (cosign-installer `6e04d2...`, setup-buildx `f95db5...`, login-action `343f7c...`, metadata-action `96383f...`, build-push-action `0565240...`).
- ✅ No `pull_request_target` / `workflow_run` — the `pull_request` trigger has no secrets exposure beyond `GITHUB_TOKEN` (read-only packages on PR... note `packages: write` is granted even on PR jobs — tighten with a `if: github.event_name == 'push'` scope or separate job permissions).
- ✅ The only `run:` step (`cosign sign`) uses the recommended **env-var indirection** (`env: TAGS: ${{ steps.meta.outputs.tags }}`) → script-injection mitigated.
- ✅ `id-token: write` scoped for sigstore; image signing enabled.

**Issues:**
1. **M7** — `actions/checkout@v3` unpinned moving tag, EOL (Node 16). Pin to v4 SHA.
2. **M5/M9** — No `cargo audit`, no `--locked` build, no SBOM/dependency review step.
3. **M9** — No Dependabot config; SHA-pinned actions will drift (comments carry versions, but no automated update).
4. Low — No `concurrency:` group or `timeout-minutes` (runaway CI, cache thrash).
5. Low — `cosign-release: "v2.1.1"` is pinned to an old release (installer supports `v2.4.x`); cosmetic, but stale tooling.
6. H5 — No `JWT_SECRET` handling anywhere; the published image contains the hardcoded key (C1) — the pipeline *blesses* the vulnerability by design.

---

## 5. Other Observations

- **Timing attacks:** signature comparison constant-time (ring 0.17.8) ✅; but the differentiated error oracle (M2) + absence of rate limiting (M3) turns `/secret-view` into a practical oracle endpoint.
- **TLS:** none in app or compose (H4) — bearer tokens sniffable in transit.
- **Security headers:** missing `Cache-Control: no-store` on token responses (M8).
- **Actix vs Axum parity:** identical issues in both implementations (shared `jwt-lib`); actix binary is also published via the same pipeline (actix Dockerfile/workflow not in this repo's scope but same pattern).
- **Functional:** both apps bind loopback only — safe by default locally; exposure is a compose/CI decision.

---

## 6. Prioritized Remediation Roadmap

### P0 — Do immediately (blocking; before any deploy/publish)
1. **C1** — Remove hardcoded key: `JWT_SECRET` env var (≥32 random bytes), fail-fast startup check, purge from git history, rotate key, overwrite ghcr images.
2. **C2** — Close the unauthenticated minting endpoint: require real client authentication + validated identity server-side; rate-limit.
3. **H3** — Non-root runtime + minimal runtime image (distroless/slim + `USER`), stop publishing images until 1–2 done.

### P1 — Within first sprint
4. **H1** — Full validation hardening: explicit `Validation::new(HS256)` + `iss`/`aud`/`sub` + emit corresponding claims.
5. **H2** — Leeway ≤ 30 s; rework TTL policy (15–30 min + refresh) .
6. **M1** — Decode into `Claims`; keep `User` as post-validation domain type; add negative tests (exp missing/expired/alg=none/RS256).
7. **M2/M8** — Generic error responses, log details server-side; add `Cache-Control: no-store`; define 401/403 semantics.
8. **M3/M4** — Rate limiting + `jti`/`iat`/`nbf` claims + denylist for revocation.

### P2 — Within first month
9. **H4/H5** — TLS termination + restrict port binding; wire `JWT_SECRET` through compose/CI secret delivery; add docker hardening flags (`cap_drop`, `read_only`, `no-new-privileges`).
10. **M5/M9** — Commit `Cargo.lock`, add workspace root, `cargo audit`/`cargo deny` in CI, update crates (`jsonwebtoken` 9.3+, `axum` latest 0.7.x/0.8).
11. **M6/L4** — Expand `.dockerignore`; fix Dockerfile COPY/workspace layout; healthcheck + restart policy.

### P3 — Hardening backlog
12. **M7** — Pin `actions/checkout@v4` SHA; Dependabot for actions + cargo; `concurrency` + `timeout-minutes`.
13. **L1/L3** — Key length/format validation + `kid` rotation table; robust `Bearer` parser; negative unit tests for all JWT paths (test coverage currently **0 %** for `jwt-lib` — see QA note below).

---

## 7. QA Note (test coverage)

- No `#[cfg(test)]` in `jwt-lib`, `axum-auth`, or `actix-auth`; no CI test job. Critical security paths (verify, validation, header parsing) have **0 % automated coverage**.
- **DoD block:** acceptance for any security remediation must include unit tests: expired-token rejection, tampered-signature rejection, `alg=none`/RS256 rejection, missing/malformed `exp` rejection, wrong `iss`/`aud` rejection, correct-token acceptance, and leeway boundary tests (±1 s).
