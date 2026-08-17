# rust_jwt — Documentation Hub

This documentation set covers the `rust_jwt` workspace: a Rust API workspace with Docker containers and JWT authentication. The workspace contains three crates — `jwt-lib` (shared JWT library), `axum-auth` (Axum web app), and `actix-auth` (Actix-web web app) — each exposing the same four HTTP endpoints.

All statements in these documents were verified against the working tree, git history, and build/security analysis reports. Where the code contains known defects, they are documented explicitly rather than glossed over.

## Document Map

| Doc | Audience | Content |
|-----|----------|---------|
| [01-architecture.md](01-architecture.md) | Developers, architects | Workspace layout, crate responsibilities, JWT lifecycle, per-framework request lifecycles, design decisions, known architectural issues |
| [02-api.md](02-api.md) | API consumers, developers | Complete endpoint reference for both apps (methods, paths, request/response JSON, errors), token structure, example curl flows |
| [03-security.md](03-security.md) | Security reviewers, maintainers | Security model, all 19 findings (2 Critical, 5 High, 8 Medium, 4 Low), JWT deep dive, prioritized remediation roadmap |
| [04-deployment.md](04-deployment.md) | DevOps, maintainers | Dockerfile analysis, docker-compose services and ports, GitHub Actions pipeline, image naming, deployment steps, known blockers |
| [05-development.md](05-development.md) | Contributors | Prerequisites, build/check/test/clippy commands, current build status (passes, 0 tests, 5 warnings), conventions, recommended fixes |

## Quickstart Pointer

The fastest way to run the axum app is Docker:

```bash
docker compose up --build
curl http://localhost:2323/
```

To run either app locally with cargo, or to exercise the JWT flow with curl, see the [README](../README.md) Quickstart section.

## Reading Order

1. README — orientation and quickstart.
2. [01-architecture.md](01-architecture.md) — how the code is organized and how a request flows.
3. [02-api.md](02-api.md) — what the API looks like on the wire.
4. [03-security.md](03-security.md) — why the workspace is not production-ready and what to fix first.
5. [04-deployment.md](04-deployment.md) — how the container pipeline works today and where it breaks.
6. [05-development.md](05-development.md) — how to build, test, and contribute.

## Repository Facts

- Git: branch `main`, 41 commits, author Samuel_Ricardo, MIT License (2024)
- Toolchain verified: cargo/rustc 1.76.0; Docker images pinned to `rust:1.88-slim` (build verified 2026-08-17)
- Build status: `cargo check`/`build` pass; 0 tests; 5 warnings (see 05-development.md)
- Security status: 19 findings — 2 Critical, 5 High, 8 Medium, 4 Low (see 03-security.md)
