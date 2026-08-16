# rust_jwt

Rust API workspace with Docker containers and JWT authentication. A shared JWT library powers two framework implementations (Axum and Actix-web) exposing the same four endpoints, containerized with Docker and published to GHCR by GitHub Actions.

## Features

- Shared JWT library (`jwt-lib`) — HS256 signing and verification with a 1-minute token TTL
- Same authentication implemented in two frameworks: Axum 0.7 and Actix-web 4.5
- Public endpoints plus a protected endpoint enforced by an `Auth` extractor in both apps
- Docker: `Dockerfile.axum` and `docker-compose.yaml` (axum image)
- CI: GitHub Actions workflow builds, pushes, and cosign-signs the axum image to GHCR
- MIT licensed

## Architecture

| Crate | Type | Role |
|-------|------|------|
| `jwt-lib` | library | Shared JWT logic: `get_jwt_secret()` (sign), `decode_jwt()` (verify), `User` / `Claims` models |
| `axum-auth` | binary | Axum 0.7 web app, binds `localhost:2323`, Tokio multi-threaded (6 workers) |
| `actix-auth` | binary | Actix-web 4.5 web app, binds `127.0.0.1:8080`, 4 OS workers |

Both binaries expose the same four routes; only the JWT logic lives in `jwt-lib`.

## Quickstart

### Option A — Docker (axum only)

```bash
docker compose up --build

curl http://localhost:2323/
```

### Option B — Local (cargo)

```bash
# axum app (port 2323)
cargo run -p axum-auth

# actix app (port 8080) — second terminal
cargo run -p actix-auth
```

### Try the flow

```bash
# 1. Public endpoint
curl http://localhost:2323/public-view

# 2. Obtain a token (endpoint is NOT authenticated — see Security status)
curl -X POST http://localhost:2323/get-token \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com"}'

# 3. Access the protected endpoint
curl http://localhost:2323/secret-view \
  -H "Authorization: Bearer <token>"
```

Use port 8080 for the actix app.

## API endpoints

| Method | Path | Auth required | Purpose |
|--------|------|---------------|---------|
| GET | `/` | No | Hello response |
| GET | `/public-view` | No | Public data |
| POST | `/get-token` | No | Issues a signed JWT |
| GET | `/secret-view` | Yes — `Authorization: Bearer <jwt>` | Protected data, echoes the token's email |

Full reference: [docs/02-api.md](docs/02-api.md).

## Security status

**Not production-ready.** A security review found 19 findings: **2 Critical, 5 High, 8 Medium, 4 Low** — including a hardcoded JWT secret (`"mykey"`) and an unauthenticated token-minting endpoint. Do not deploy or publish images before remediating the Critical items.

Details and remediation roadmap: [docs/03-security.md](docs/03-security.md).

## Documentation

| Doc | Content |
|-----|---------|
| [docs/index.md](docs/index.md) | Documentation hub and doc map |
| [docs/01-architecture.md](docs/01-architecture.md) | Workspace layout, crate responsibilities, JWT and request lifecycles, design decisions |
| [docs/02-api.md](docs/02-api.md) | Full API reference for both apps, token structure, curl examples |
| [docs/03-security.md](docs/03-security.md) | Security model, all findings, JWT deep dive, remediation roadmap |
| [docs/04-deployment.md](docs/04-deployment.md) | Dockerfile, docker-compose, GitHub Actions pipeline, deployment steps |
| [docs/05-development.md](docs/05-development.md) | Prerequisites, build/check/test commands, current status, conventions |

## License

MIT License — Copyright (c) 2024 Samuel_Ricardo. See [LICENSE](LICENSE).
