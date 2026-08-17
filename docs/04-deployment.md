# 04 — Deployment

This document covers the container and CI/CD story of the workspace: `Dockerfile.axum`, `docker-compose.yaml`, and the GitHub Actions workflow `.github/workflows/docker-publish-axum.yml`. It also documents known blockers that must be fixed before the pipeline can be considered production-safe.

## 1. Dockerfile.axum

The current file (verbatim behavior):

```dockerfile
FROM rust:1.88-slim AS build

WORKDIR /app

COPY ./jwt-lib/ /jwt-lib/
COPY ./axum-auth/ .

RUN cargo build --release

FROM rust:1.88-slim

WORKDIR /usr/local/bin

COPY --from=build /app/target/release/axum-auth .

EXPOSE 2323

CMD ["./axum-auth"]
```

Structure:

| Aspect | Detail |
|--------|--------|
| Stages | Two: builder (`rust:1.88-slim`) and runtime (`rust:1.88-slim`) |
| Build | `cargo build --release` for the single `axum-auth` package |
| Artifact | `target/release/axum-auth` staged to `/usr/local/bin/axum-auth` |
| Port | `EXPOSE 2323` |
| Command | `CMD ["./axum-auth"]` |

### 1.1 Known defects

1. **`COPY ./jwt-lib/ ../` regression — FIXED** (commit `599107f`): now `COPY ./jwt-lib/ /jwt-lib/`, so jwt-lib's contents land in `/jwt-lib/` where the `../jwt-lib` path dependency resolves. Verified with `docker build` on 2026-08-17. A workspace-aware build (`COPY . .` + root `Cargo.toml`) remains the cleaner long-term option.
2. **Runtime runs as root with the full Rust toolchain** — toolchain updated to `rust:1.88-slim` (commit `599107f`) but the image still ships cargo/rustc/git and still has no `USER` directive (security finding H3). The runtime stage should be a minimal image (debian slim/distroless/static musl + scratch) with a non-root user.
3. **Base image not digest-pinned** — `rust:1.88-slim` is a moving tag; CI and local builds can diverge.
4. **No `HEALTHCHECK`**, no `--locked` build (impossible while `Cargo.lock` is gitignored).
5. **The root workspace manifest is never copied into the image** — container builds are single-package builds. This incidentally masks the lowercase `cargo.toml` filename problem, which would break a workspace build on Linux.

There is no `Dockerfile.actix` in the working tree or in git history since commit `704822a` (deletion committed 2026-08-17). The actix containerization was removed in the consolidation commit `1dd6df8`.

## 2. docker-compose.yaml

```yaml
version: "3.8"

services:
  axum_auth:
    build:
      context: .
      dockerfile: Dockerfile.axum
    ports:
      - "2323:2323"
```

| Service | Build | Host:Container ports | Notes |
|---------|-------|----------------------|-------|
| `axum_auth` | `Dockerfile.axum` (context `.`) | `2323:2323` | The only service today |

The pre-consolidation revision also defined an `actix_auth` service (`8080:8080`), removed in commit `1dd6df8`.

### 2.1 Known defects

1. **Dead port mapping (finding L4)** — `2323:2323` publishes on `0.0.0.0`, but the app binds `localhost` *inside* the container, so the published port cannot reach the process. The mapping advertises external exposure (without TLS, finding H4) while the service is actually unreachable.
2. **No secret delivery (finding H5)** — no `environment:` for `JWT_SECRET`; even after moving the key to an env var, compose must pass it (`environment: JWT_SECRET: ${JWT_SECRET}` + gitignored `.env`, or Docker secrets).
3. **Missing hardening** — no `cap_drop`, `security_opt: no-new-privileges`, `read_only`, `tmpfs`, `restart`, resource limits, or `healthcheck`. `version: "3.8"` is obsolete (compose spec).

## 3. GitHub Actions Pipeline (docker-publish-axum.yml)

This is the only workflow. Pipeline steps in order:

| # | Step | Action | Runs on |
|---|------|--------|---------|
| 1 | Checkout repository | `actions/checkout@11d5960a326750d5838078e36cf38b85af677262` # v4.4.0 (pinned SHA — M7 FIXED in `445f859`) | always |
| 2 | Install cosign | `sigstore/cosign-installer` @ SHA (v3.1.1), `cosign-release: v2.1.1` | not on PR |
| 3 | Set up Docker Buildx | `docker/setup-buildx-action` @ SHA (v3.0.0) | always |
| 4 | Log into registry | `docker/login-action` @ SHA (v3.0.0), registry `ghcr.io`, user `${{ github.actor }}`, password `${{ secrets.GITHUB_TOKEN }}` | not on PR |
| 5 | Extract Docker metadata | `docker/metadata-action` @ SHA (v5.0.0), images `ghcr.io/${{ github.repository }}` | always |
| 6 | Build and push image | `docker/build-push-action` @ SHA (v5.0.0), `context: .`, `file: Dockerfile.axum`, `push` only off-PR, GHA cache (`type=gha`, `mode=max`) | always |
| 7 | Sign the published image | `run:` cosign sign (env-var indirection), using OIDC `id-token` | not on PR |

Triggers: push to `main`, tags `v*.*.*`, pull requests against `main`.

### 3.1 What the pipeline does — and does not do

- Builds **only the axum image** (`file: Dockerfile.axum`). The actix workflow (`docker-publish.yml`) was deleted in commit `1dd6df8`.
- Pushes to **GHCR** on push/tag (never on PR); tags derived by `docker/metadata-action` (branch tag `main`, semver `v*.*.*` for releases).
- Signs the image with **cosign** (sigstore) on push/tag.
- **No test, check, clippy, or audit steps** — build and ship only.
- All 6 third-party actions are SHA-pinned (`actions/checkout` pinned to v4.4.0 SHA since `445f859`).

## 4. Image Naming

| Variable | Value |
|----------|-------|
| `REGISTRY` | `ghcr.io` |
| `IMAGE_NAME` | `${{ github.repository }}` → `<owner>/<repo>` |
| Full image | `ghcr.io/<owner>/<repo>` (for this repository: `ghcr.io/Samuel-Ricardo/rust_jwt`) |
| Tags | `main` (branch), `v*.*.*` (semver release tags) |

## 5. How to Deploy

### 5.1 Local container run (axum)

```bash
# Build and start via compose
docker compose up --build

# Verify the container built and the port mapping exists
docker compose ps
```

Note: with the current code, the published port cannot reach the loopback-bound process (L4). Verify with `docker compose exec axum_auth curl http://localhost:2323/` or fix the bind first.

### 5.2 Manual image build

```bash
docker build -f Dockerfile.axum -t rust_jwt-axum:local .
docker run --rm -p 2323:2323 rust_jwt-axum:local
```

### 5.3 Pull the published GHCR image

After a CI run on `main` (the build pipeline was verified locally with `docker build` on 2026-08-17):

```bash
docker pull ghcr.io/Samuel-Ricardo/rust_jwt:main
```

### 5.4 Production checklist (all currently unmet)

1. Fix C1/C2 (secret from env; authenticated minting) — see 03-security.md.
2. ~~Fix the Dockerfile COPY regression~~ — done (`599107f`); add a root `Cargo.toml` for a workspace-aware build.
3. Run the runtime as a non-root user on a minimal image; pin base images by digest.
4. Pass `JWT_SECRET` via compose `environment:` (`.env` gitignored) or secrets manager.
5. Add TLS termination, restrict port binding, and Docker hardening flags.
6. Expand `.dockerignore` (exclude `.git`, `.github`, `*.md`, `.env*`).
7. Add CI quality gates: `cargo test`, `cargo clippy`, `cargo audit`.

## 6. CI Pipeline Diagram

```mermaid
flowchart LR
    A[push to main / tag v*.*.* / PR] --> B[Checkout repo]
    B --> C[Install cosign]
    C --> D[Set up Docker Buildx]
    D --> E[Log into ghcr.io]
    E --> F[Extract metadata: tags + labels]
    F --> G[Build and push image: Dockerfile.axum]
    G --> H[Sign image with cosign]
    H --> I[GHCR: ghcr.io/owner/repo]