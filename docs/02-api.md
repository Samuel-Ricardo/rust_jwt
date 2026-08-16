# 02 — API Reference

This document is the complete HTTP API reference for both applications. The route set is identical in `axum-auth` and `actix-auth`; only the base URL, the port, and a few error semantics differ.

## 1. Servers

| App | Framework | Base URL | Bind | Runtime |
|-----|-----------|----------|------|---------|
| axum-auth | Axum 0.7 | `http://localhost:2323` | `localhost:2323` (loopback) | Tokio, 6 worker threads |
| actix-auth | Actix-web 4.5 | `http://127.0.0.1:8080` | `127.0.0.1:8080` (loopback) | actix runtime, 4 OS workers |

Both servers bind loopback only; they are not reachable from other hosts or containers without port mapping.

## 2. Conventions

- All responses are `Content-Type: application/json`.
- Most responses use the envelope `{"success": bool, "data": ...}`; the only exception is axum `GET /`, which returns `{"message": "Hello, World!"}`.
- `success: true` means the request succeeded; `success: false` is used on application-level errors.
- No response headers are set for caching, CORS, or security (`Cache-Control: no-store` is absent on token responses).

## 3. Endpoint Summary

| Method | Path | Auth required | Handler (both apps) | Purpose |
|--------|------|---------------|---------------------|---------|
| GET | `/` | No | `hello_world` | Hello response |
| GET | `/public-view` | No | `public_view_handler` | Public data |
| POST | `/get-token` | No | `get_token_handler` | Issues a signed JWT |
| GET | `/secret-view` | Yes — `Authorization: Bearer <jwt>` | `secret_view_handler` | Protected data; echoes the token's email |

## 4. Endpoint Details

### 4.1 GET /

Public. Returns a hello message.

| App | Status | Response body |
|-----|--------|---------------|
| axum-auth | 200 OK | `{"message": "Hello, World!"}` |
| actix-auth | 200 OK | `{"success": true, "data": "Hello World!"}` |

Note the response-shape inconsistency between the two apps (see 01-architecture.md §5.4).

### 4.2 GET /public-view

Public. Returns static data visible to all users.

| Status | Response body |
|--------|---------------|
| 200 OK | `{"success": true, "data": {"message": "This data is visible to all users"}}` |

### 4.3 POST /get-token

Public — **no client authentication is performed** (see 03-security.md, finding C2). Issues a signed JWT for any submitted email.

Request body (JSON):

```json
{"email": "user@example.com"}
```

The body is deserialized into `jwt-lib::model::user::User` (`{ "email": String }`).

| Status | Response body |
|--------|---------------|
| 200 OK | `{"success": true, "data": {"token": "<jwt>"}}` |
| 400 Bad Request | `{"success": false, "data": {"message": "<error string>"}}` (e.g. if signing fails) |
| 422 Unprocessable Entity | axum `Json` rejection (malformed JSON or missing/extra fields) |
| 400 Bad Request | actix `Json` rejection (malformed JSON or missing/extra fields) |

Example:

```bash
curl -X POST http://localhost:2323/get-token \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com"}'

# Response:
# {"success": true, "data": {"token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJlbWFpbCI6..."}}
```

### 4.4 GET /secret-view

Protected. Requires a valid JWT in the `Authorization` header. Returns the email embedded in the token.

Request header:

```
Authorization: Bearer <jwt>
```

| Status | Response body |
|--------|---------------|
| 200 OK | `{"success": true, "data": {"email": "<email from token>"}}` |
| 401 Unauthorized | `{"success": false, "data": {"message": "No token provided"}}` — header missing or no 2nd space-separated segment |
| 401 Unauthorized | `{"success": false, "data": {"message": "<jsonwebtoken error string>"}}` — invalid/expired/tampered token (e.g. `ExpiredSignature`, `InvalidSignature`, `InvalidToken`) |

Note: the `Bearer` scheme is never verified — any header whose second space-separated segment is a valid JWT is accepted (see 03-security.md, finding L3). The raw jsonwebtoken error string is echoed to the client (finding M2).

Example:

```bash
curl http://localhost:2323/secret-view \
  -H "Authorization: Bearer <token>"

# Response:
# {"success": true, "data": {"email": "user@example.com"}}
```

## 5. Token Structure

Tokens are HS256-signed JWTs produced by `jwt-lib`.

| Component | Value |
|-----------|-------|
| Header | `{"alg": "HS256", "typ": "JWT"}` (`Header::default()` from jsonwebtoken) |
| Payload | `{"email": "<string>", "exp": <unix timestamp>}` — only two claims |
| Signature | HMAC-SHA256 over header.payload with the secret `"mykey"` (hardcoded) |
| TTL | 60 seconds (`exp = Utc::now() + Duration::minutes(1)`) |
| Effective validity | up to ~2 minutes (60 s TTL + 60 s validation leeway) |

Claims table:

| Claim | Type | Present | Validated | Notes |
|-------|------|---------|-----------|-------|
| `email` | string | Yes | No (echoed, not validated) | Identity claim; any string accepted at mint time |
| `exp` | i64 (unix seconds) | Yes | Yes (60 s leeway) | Expiration; missing `exp` is rejected |
| `iss`, `aud`, `sub`, `nbf`, `iat`, `jti` | — | No | No | Not emitted and not validated |

## 6. Example JWT Flow (curl)

```bash
BASE=http://localhost:2323        # use http://127.0.0.1:8080 for actix-auth

# 1. Public endpoints
curl $BASE/
curl $BASE/public-view

# 2. Obtain a token
TOKEN=$(curl -s -X POST $BASE/get-token \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com"}' \
  | jq -r '.data.token')

# 3. Access the protected endpoint with the token
curl $BASE/secret-view -H "Authorization: Bearer $TOKEN"

# 4. Negative cases
curl -i $BASE/secret-view                        # 401 No token provided
curl -i $BASE/secret-view -H "Authorization: Bearer not.a.jwt"   # 401 InvalidToken
curl -i $BASE/get-token -H "Content-Type: application/json" -d '{}'  # 422 (axum) / 400 (actix)
```

## 7. Framework-Specific Differences

| Aspect | axum-auth | actix-auth |
|--------|-----------|------------|
| Base URL | `http://localhost:2323` | `http://127.0.0.1:8080` |
| `GET /` body | `{"message": "Hello, World!"}` | `{"success": true, "data": "Hello World!"}` |
| Malformed JSON body on `/get-token` | 422 | 400 |
| Protected-endpoint errors | 401 with JSON body (rejection is the response) | 401 with JSON body via `InternalError` |

All other request/response shapes are identical.
