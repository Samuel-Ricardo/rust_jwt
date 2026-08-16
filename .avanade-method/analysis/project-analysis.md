# Rust JWT Workspace — Comprehensive Analysis Report

**Workspace root:** `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt`
**Git:** branch `main`, 41 commits, author `Samuel_Ricardo`, MIT License (2024). No CI status/commits after 2024-03-17.
**Toolchain observed on machine:** rustc/cargo 1.76.0; Docker images pinned to `rust:1.71-slim`.

This is a JWT authentication demo workspace with 3 crates: `jwt-lib` (shared library), `axum-auth` (Axum web app), `actix-auth` (Actix web app). All code quotes are verbatim from the working tree; files deleted from the working tree but present in git history were recovered via `git show` and are marked as such.

---

## 1. Workspace Structure

### 1.1 File Inventory (working tree, excluding `target/`)

```
C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\
├── cargo.toml                          ← NOTE: lowercase filename (unusual)
├── Cargo.lock                          ← present locally, but gitignored (NOT committed)
├── README.md                           ← 2 lines: "Rust API with Docker Containers and JWT Authentication"
├── LICENSE                             ← MIT, Copyright (c) 2024 Samuel_Ricardo
├── .gitignore
├── .dockerignore
├── Dockerfile.axum
├── docker-compose.yaml
├── .github\workflows\docker-publish-axum.yml
├── jwt-lib\                            ← shared library crate
│   ├── Cargo.toml
│   └── src\
│       ├── lib.rs
│       └── model\{mod.rs, user.rs, claims.rs}
├── axum-auth\                          ← Axum web app crate
│   ├── Cargo.toml
│   └── src\{main.rs, server.rs, controller\{mod.rs, auth.rs}, middleware\{mod.rs, auth.rs}}
└── actix-auth\                         ← Actix web app crate
    ├── Cargo.toml
    └── src\{main.rs, server.rs, controller\{mod.rs, auth.rs}, middleware\{mod.rs, auth.rs}}
```

**Files in git HEAD but missing from working tree (uncommitted deletions):**
- `Dockerfile.actix` — committed at HEAD (`23200c3`), deleted in working tree (`git status` shows ` D Dockerfile.actix`).
- `.github/workflows/docker-publish.yml` (actix variant) — deleted in HEAD commit `1dd6df8` (see §7).

### 1.2 Root Manifest — `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\cargo.toml`

```toml
[workspace]
members = ['axum-auth', 'actix-auth', "jwt-lib"]
resolver = "2"
```

- **Workspace members:** `axum-auth`, `actix-auth`, `jwt-lib` (all path members; no `[workspace.dependencies]`).
- **No `[workspace.package]`**, no `[profile.*]` overrides, no features defined anywhere.
- **Quirk:** the file is named `cargo.toml` (all lowercase). This works on Windows/macOS (case-insensitive FS) but **breaks builds on case-sensitive filesystems (Linux/CI)** unless a `Cargo.toml` exists. The Docker build avoids this only because it never copies the workspace manifest (see §7).
- **`Cargo.lock` is gitignored** (`.gitignore` line 8), so the workspace is not reproducible across clones. For a workspace with binaries, best practice would be to commit it.

### 1.3 Crate Manifests

**`C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\jwt-lib\Cargo.toml`** — library, edition 2021:
```toml
[dependencies]
chrono = "0.4.35"
jsonwebtoken = "9.2.0"
serde = { version = "1.0.197", features = ["derive"] }
```

**`C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\axum-auth\Cargo.toml`** — binary, edition 2021:
```toml
[dependencies]
axum = "0.7.4"
serde_json = "1.0.114"
tokio = { version = "1.36.0", features = ["full"] }
jwt-lib = { path = "../jwt-lib" }
```

**`C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\actix-auth\Cargo.toml`** — binary, edition 2021:
```toml
[dependencies]
actix-web = "4.5.1"
serde_json = "1.0.114"
jwt-lib = { path = "../jwt-lib" }
```

### 1.4 Resolved Versions (from `Cargo.lock`)

| Package | Resolved |
|---|---|
| axum / axum-core | 0.7.4 / 0.4.3 |
| actix-web / actix-http | 4.5.1 / 3.6.0 |
| jsonwebtoken | 9.2.0 |
| chrono | 0.4.35 |
| serde | 1.0.197 |
| serde_json | 1.0.114 |
| tokio | 1.36.0 (features = "full") |

### 1.5 `.gitignore` (`C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\.gitignore`)
Ignores `debug/`, `target/`, `Cargo.lock`, `**/*.rs.bk`, `*.pdb`. The header comment acknowledges Cargo.lock should be kept for executables — but it isn't.

### 1.6 `.dockerignore` (`C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\.dockerignore`)
Single line: `**/target`. (The local `target/` dir exists and is excluded from build context.)

---

## 2. `jwt-lib` Crate (shared JWT library)

### 2.1 Public API Surface

**`C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\jwt-lib\src\lib.rs`** — the entire library:

```rust
pub mod model;

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

use model::claims::Claims;
use model::user::User;

pub fn get_jwt_secret(user: User) -> Result<String, String> {
    let token = encode(
        &Header::default(),
        &Claims {
            email: user.email,
            exp: (Utc::now() + Duration::minutes(1)).timestamp(),
        },
        &EncodingKey::from_secret("mykey".as_bytes()),
    )
    .map_err(|e| e.to_string());

    return token;
}

pub fn decode_jwt(token: &str) -> Result<User, String> {
    let token_data = decode::<User>(
        token,
        &DecodingKey::from_secret("mykey".as_bytes()),
        &Validation::default(),
    );

    match token_data {
        Ok(token_data) => Ok(token_data.claims),
        Err(e) => Err(e.to_string()),
    }
}
```

Public functions:
- `pub fn get_jwt_secret(user: User) -> Result<String, String>` — consumes `User` by value, returns signed JWT string.
- `pub fn decode_jwt(token: &str) -> Result<User, String>` — validates + decodes, returns the embedded `User`.

Also public: `pub mod model` (with `model::claims::Claims`, `model::user::User`).

### 2.2 Data Structures

**`C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\jwt-lib\src\model\user.rs`:**
```rust
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct User {
    pub email: String,
}
```

**`C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\jwt-lib\src\model\claims.rs`:**
```rust
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Claims {
    pub email: String,
    pub exp: i64,
}
```

**`C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\jwt-lib\src\model\mod.rs`:** `pub mod claims; pub mod user;`

Notes:
- Only `Serialize`/`Deserialize` derived — **no `Debug`, `Clone`, `PartialEq`** on either struct (matters for testing and for `Auth` extractor ergonomics).
- Both structs have identical fields except `Claims` adds `exp: i64` (Unix epoch seconds).

### 2.3 JWT Functionality Implemented

| Aspect | Implementation |
|---|---|
| **Signing** | `encode` with `Header::default()` → **HS256** (`jsonwebtoken` 9.2.0) |
| **Key** | `EncodingKey::from_secret("mykey".as_bytes())` — **hardcoded string** |
| **Expiration** | `exp = Utc::now() + Duration::minutes(1)` → **1-minute TTL** |
| **Verification** | `decode` with `Validation::default()` |
| **Algorithms accepted** | HS256 only (default) |
| **Claims validated** | `exp` only — `Validation::default()` sets `required_spec_claims = ["exp"]`, `validate_exp = true`, `leeway = 60` (seconds) |
| **Not validated** | `nbf`, `iss`, `aud`, `sub` — all default to `false`/`None`; no required private claims |
| **Issuer/audience claims** | Not present in `Claims` struct at all |

### 2.4 Validation Semantics & Notable Quirk

**Signature verification** — always enforced by `jsonwebtoken::decode`; the `DecodingKey::from_secret("mykey")` must match the signing key or `decode` returns `Err("InvalidSignature")`.

**Expiration check** — enforced at decode time with 60 s default leeway. Combined with the 1-minute TTL, a token is effectively valid for up to ~2 minutes from issuance (1 min + 60 s leeway). A token missing `exp` is rejected (`MissingRequiredClaim("exp")`).

**Critical design quirk — decode into `User`, not `Claims`:**

```rust
let token_data = decode::<User>(token, ...);
```

Tokens are *encoded* as `Claims { email, exp }` but *decoded* into `User { email }`. This works only because:
1. `serde` ignores the unknown `exp` field when deserializing into `User` (no `#[serde(deny_unknown_fields)]`).
2. `exp` is still validated independently by `Validation::default()` before deserialization completes.

Consequence: the decoded `User` returned by `decode_jwt` never exposes `exp`, and the `Claims` struct is only used at encode time. If `deny_unknown_fields` were ever added to `User`, `decode_jwt` would break.

### 2.5 Config Options

**None.** No environment variables, no `Config` struct, no constructor. The secret, algorithm, and TTL are all hardcoded inside `lib.rs`:

- Secret: literal `"mykey"` (both `EncodingKey` and `DecodingKey`).
- TTL: `Duration::minutes(1)`.
- Algorithm: implicit via `Header::default()` (HS256).

---

## 3. `axum-auth` Crate Deep Dive

### 3.1 File Inventory

| File | Role |
|---|---|
| `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\axum-auth\src\main.rs` | Binary entry point |
| `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\axum-auth\src\server.rs` | Router + listener setup |
| `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\axum-auth\src\controller\mod.rs` | `hello_world`, `public_view_handler` |
| `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\axum-auth\src\controller\auth.rs` | `get_token_handler`, `secret_view_handler` |
| `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\axum-auth\src\middleware\mod.rs` | `pub mod auth;` |
| `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\axum-auth\src\middleware\auth.rs` | `Auth` extractor (JWT validation) |

### 3.2 Entry Point — `src/main.rs`

```rust
mod controller;
mod middleware;
mod server;

fn main() {
    server::startup();
}
```

Note: `server::startup()` is `async` and carries its own `#[tokio::main]` attribute (see below) — calling it from a plain `fn main()` works because `#[tokio::main]` transforms `startup` into a blocking entry. This is an unusual pattern (the macro normally decorates `main` itself).

### 3.3 Server Setup & Routes — `src/server.rs`

```rust
#[tokio::main(flavor = "multi_thread", worker_threads = 6)]
pub async fn startup() {
    let routes = Router::new()
        .route("/", get(hello_world))
        .route("/public-view", get(public_view_handler))
        .route("/get-token", post(get_token_handler))
        .route("/secret-view", get(secret_view_handler));

    let tcp_listener = TcpListener::bind("localhost:2323")
        .await
        .expect("Address should be free and valid");

    axum::serve(tcp_listener, routes)
        .await
        .expect("Error serving application");
}
```

**Runtime:** Tokio multi-threaded, 6 worker threads, bound to `localhost:2323` (loopback only — not reachable from other hosts/containers without port mapping).

**Endpoint table (identical route set in both apps):**

| Method | Path | Handler | Auth required |
|---|---|---|---|
| GET | `/` | `hello_world` | No |
| GET | `/public-view` | `public_view_handler` | No |
| POST | `/get-token` | `get_token_handler` | No (issues token) |
| GET | `/secret-view` | `secret_view_handler` | **Yes — `Auth` extractor** |

### 3.4 Public Handlers — `src/controller/mod.rs`

```rust
pub async fn hello_world() -> Response<String> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body("{\"message\": \"Hello, World!\"}".to_string())
        .unwrap_or_default()
}

pub async fn public_view_handler() -> Response<String> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(
            json!({
                "success": true,
                "data": {
                    "message": "This data is visible to all users"
                }
            })
            .to_string(),
        )
        .unwrap_or_default()
}
```

**Inconsistency quirk:** `hello_world` returns `{"message": "Hello, World!"}` without the `success`/`data` envelope used by every other endpoint (and by the actix twin, which returns `{"success": true, "data": "Hello World!"}`).

**Response pattern:** responses are hand-built `axum::response::Response<String>` with `Response::builder()...unwrap_or_default()` — no `Json` extractor used for responses, JSON is stringified manually.

### 3.5 Auth Handlers — `src/controller/auth.rs`

**Issue token (request → response):**

```rust
pub async fn get_token_handler(Json(user): Json<User>) -> Response<String> {
    let token = jwt_lib::get_jwt_secret(user);

    match token {
        Ok(token) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(json!({ "success": true, "data": { "token": token } }).to_string())
            .unwrap_or_default(),

        Err(error) => Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header(header::CONTENT_TYPE, "application/json")
            .body(json!({ "success": false, "data": { "message": error } }).to_string())
            .unwrap_or_default(),
    }
}
```

**Protected handler — note the `Auth` extractor as a function argument:**

```rust
pub async fn secret_view_handler(Auth(user): Auth) -> Response<String> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(json!({ "success": true, "data": user }).to_string())
        .unwrap_or_default()
}
```

**Request/response shapes:**

| Endpoint | Request body | Success (200) | Error |
|---|---|---|---|
| `POST /get-token` | `{"email": "user@example.com"}` | `{"success": true, "data": {"token": "<jwt>"}}` | 400 `{"success": false, "data": {"message": "<err>"}}` |
| `GET /secret-view` | `Authorization: Bearer <jwt>` | `{"success": true, "data": {"email": "..."}}` | 401 `{"success": false, "data": {"message": "<err>|No token provided"}}` |

Malformed JSON body → axum's default `Json` rejection (**422 Unprocessable Entity**).

### 3.6 Middleware (JWT Validation) — `src/middleware/auth.rs` — **the critical code**

**Architecture note:** this is **not** a Tower `Layer`/`middleware::from_fn` chain. It is a **request extractor** (`FromRequestParts`) — auth is enforced *per-handler* by declaring `Auth(user): Auth` as a handler parameter, not globally on the router.

```rust
pub struct Auth(pub User);

#[async_trait]
impl<S> FromRequestParts<S> for Auth
where
    S: Send + Sync,
{
    type Rejection = Response<String>;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let access_token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|str| str.split(" ").nth(1));

        match access_token {
            Some(token) => {
                let user = jwt_lib::decode_jwt(token);

                match user {
                    Ok(user) => Ok(Auth(user)),
                    Err(err) => Err(Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(json!({
                          "success": false,
                          "data": { "message": err }
                        }).to_string())
                        .unwrap_or_default()),
                }
            }
            None => Err(Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(json!({
                  "success": false,
                  "data": { "message": "No token provided" }
                }).to_string())
                .unwrap_or_default()),
        }
    }
}
```

**Validation flow (step by step):**
1. Read `Authorization` header; fail → 401 `"No token provided"`.
2. `str.split(" ").nth(1)` — take the 2nd space-separated segment. **The `Bearer` scheme is never verified** (any 2nd token, e.g. from `Basic xyz`, is passed to the validator); literal-space split also mis-parses tabs/multiple spaces.
3. Call `jwt_lib::decode_jwt(token)` → `jsonwebtoken` verifies HS256 signature + `exp` (60 s leeway).
4. Success → wrap decoded `User` in `Auth` and inject into the handler.
5. Failure → 401 with the **raw jsonwebtoken error string** (e.g. `"ExpiredSignature"`, `"InvalidSignature"`, `"InvalidToken"`) echoed to the client — minor internal-detail disclosure.

### 3.7 State Management

**None.** No `AppState`, no `Router::with_state`, no database, no in-memory store, no configuration struct. The app is fully stateless; every request independently re-validates the token. `S: Send + Sync` bound on the extractor is the only generic state interaction.

---

## 4. `actix-auth` Crate Deep Dive

### 4.1 File Inventory

| File | Role |
|---|---|
| `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\actix-auth\src\main.rs` | Binary entry point |
| `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\actix-auth\src\server.rs` | `HttpServer` + `App` setup |
| `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\actix-auth\src\controller\mod.rs` | `hello_world`, `public_view_handler` |
| `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\actix-auth\src\controller\auth.rs` | `get_token_handler`, `secret_view_handler` |
| `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\actix-auth\src\middleware\mod.rs` | `pub mod auth;` |
| `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\actix-auth\src\middleware\auth.rs` | `Auth` extractor (JWT validation) |

### 4.2 Entry Point — `src/main.rs`

```rust
mod controller;
mod middleware;
mod server;

fn main() {
    server::setup();
}
```

### 4.3 Server Setup & Routes — `src/server.rs`

```rust
#[actix_web::main]
pub async fn setup() -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .route("/", get().to(hello_world))
            .route("/public-view", get().to(public_view_handler))
            .route("/get-token", post().to(get_token_handler))
            .route("/secret-view", get().to(secret_view_handler))
    })
    .workers(4)
    .bind("127.0.0.1:8080")
    .expect("Address should be free and valid")
    .run()
    .await
}
```

**Runtime:** `#[actix_web::main]` (actix's Tokio-based runtime), **4 OS workers** (`HttpServer` clones the `App` factory per worker), bound to `127.0.0.1:8080`.

### 4.4 Handlers — `src/controller/mod.rs` and `src/controller/auth.rs`

```rust
// controller/mod.rs
pub async fn hello_world() -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "success": true,
        "data": "Hello World!"
    }))
}

pub async fn public_view_handler() -> HttpResponse {
    HttpResponse::Ok().json(json!({
      "success": true,
      "data": { "message": "This data is visible to all users" }
    }))
}

// controller/auth.rs
pub async fn get_token_handler(Json(user): Json<User>) -> HttpResponse {
    let token = jwt_lib::get_jwt_secret(user);

    match token {
        Ok(token) => HttpResponse::Ok().json(json!({
          "success": true,
          "data": { "token": token }
        })),
        Err(error) => HttpResponse::BadRequest().json(json!({
          "success": false,
          "data": { "message": error }
        })),
    }
}

pub async fn secret_view_handler(Auth(user): Auth) -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "success": true,
        "data": user
    }))
}
```

Responses use the idiomatic `HttpResponse::Ok().json(...)` builder (vs axum's manual `Response::builder()` string bodies).

### 4.5 Middleware (JWT Validation) — `src/middleware/auth.rs` — **the critical code**

```rust
pub struct Auth(pub User);

impl FromRequest for Auth {
    type Error = InternalError<String>;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let access_token = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|str| str.split(" ").nth(1));

        match access_token {
            Some(token) => {
                let user = jwt_lib::decode_jwt(token);

                match user {
                    Ok(user) => ready(Ok(Auth(user))),
                    Err(e) => ready(Err(InternalError::from_response(
                        e.clone(),
                        HttpResponse::Unauthorized().json(json!({
                          "success": false,
                          "data": { "message": e }
                        })),
                    ))),
                }
            }
            None => ready(Err(InternalError::from_response(
                String::from("No Token Provided"),
                HttpResponse::Unauthorized().json(json!({
                  "success": false,
                  "data": { "message": "No token provided" }
                })),
            ))),
        }
    }
}
```

Validation flow is identical to axum's: header → `split(" ").nth(1)` → `decode_jwt` → wrap `Auth(user)` or 401. Errors are produced via `InternalError::from_response(cause, response)` so the JSON body is delivered as the extractor rejection while the cause string is preserved.

### 4.6 Framework Differences: actix vs axum

| Aspect | **actix-auth** (actix-web 4.5.1) | **axum-auth** (axum 0.7.4) |
|---|---|---|
| **Middleware approach** | `FromRequest` extractor — synchronous, `Ready` future (`std::future::ready`), no async work | `FromRequestParts<S>` extractor — `#[async_trait]`, async fn |
| **True middleware?** | No — extractor pattern, not `wrap_fn` / `middleware::from_fn` / `ServiceFactory` | No — extractor pattern, not Tower `Layer` / `middleware::from_fn` |
| **Error type** | `type Error = InternalError<String>`; `InternalError::from_response` carries custom JSON | `type Rejection = Response<String>` — rejection *is* the response |
| **Request access** | `&HttpRequest` + `&mut Payload` (payload unused) | `&mut Parts` + `&S` state |
| **Handler args** | `web::Json<User>`, `Auth(user): Auth` | `Json<User>`, `Auth(user): Auth` |
| **Response type** | `HttpResponse` via `HttpResponse::Ok().json()` | `Response<String>` via `Response::builder()` + `json!().to_string()` |
| **Runtime** | `#[actix_web::main]`, `.workers(4)` | `#[tokio::main(flavor = "multi_thread", worker_threads = 6)]` |
| **Server** | `HttpServer::new(App::new().route(...))` per-worker factory | `Router::new()` + `axum::serve(TcpListener, router)` |
| **Bind** | `127.0.0.1:8080` | `localhost:2323` |
| **Malformed JSON body** | 400 (actix `Json` rejection) | 422 (axum `Json` rejection) |
| **hello_world shape** | `{"success": true, "data": "Hello World!"}` | `{"message": "Hello, World!"}` (inconsistent) |

**State management:** none in either app — both are stateless, no `App::app_data`/`Router::with_state`, no database.

---

## 5. Test Coverage

### 5.1 What exists

**Zero tests in the working tree.** Verified by:
- Regex search across all `*.rs` files for `#[test]`, `#[tokio::test]`, `#[cfg(test)]`, `mod tests` → **no matches**.
- Workspace tree contains **no `tests/` directories** in any crate.
- No `#[cfg(test)]` modules anywhere.

### 5.2 Git history

The only tests ever committed were the `cargo new` boilerplate in `jwt-lib/src/lib.rs`:

```rust
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
```

- Added in `4764f9c` (`create: lib - jwt`), removed in `87c0ceb` when the real JWT functions replaced `add`.
- `git grep "#[test]" $(git rev-list --all)` confirms those are the **only** test instances in the entire history.

### 5.3 Coverage gaps (what is NOT tested)

| Area | Gap |
|---|---|
| JWT signing | No test that `get_jwt_secret` returns a well-formed 3-part token |
| JWT round-trip | No test `encode → decode` yields the same email |
| Expiration | No test that an expired token is rejected (would need injectable clock — `Utc::now()` is hardcoded) |
| Tampered token | No test that a modified token / wrong key fails |
| Middleware (axum) | No test of `Auth` extractor: missing header, malformed header, invalid token → 401 |
| Middleware (actix) | Same gaps |
| Handlers | No tests for `/get-token`, `/secret-view`, `/public-view`, `/` |
| Integration | No HTTP-level tests against a live server |

Testability blockers: hardcoded secret/TTL, no injectable clock, no `Debug`/`PartialEq` derives on `User`/`Claims` (needed for `assert_eq!` on decoded results).

---

## 6. Security-Relevant Details

### 6.1 Secret / Key Handling — **hardcoded, not env**

`C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\jwt-lib\src\lib.rs`:

```rust
&EncodingKey::from_secret("mykey".as_bytes())   // signing
&DecodingKey::from_secret("mykey".as_bytes())   // verification
```

- Literal `"mykey"` compiled into the library — **no env var, no config file, no runtime injection**.
- Same symmetric secret for signing and verification (HS256 requirement).
- Any deployment compromise of the binary exposes the key; rotating requires a rebuild. The Docker image embeds it too.
- **Recommendation:** read from `env::var("JWT_SECRET")` (or a secrets manager) at startup, and reject a missing/weak key.

### 6.2 Algorithm

- **HS256** (HMAC-SHA256) — `Header::default()` at encode; `Validation::default()` restricts accepted algorithms to HS256 at decode (prevents `alg: none` and RS/ES confusion attacks).
- HS256 is symmetric — acceptable for a single-service demo; asymmetric RS256 would be preferred for multi-service trust boundaries.

### 6.3 Token Validation Logic (decode side)

`jsonwebtoken` 9.2.0 `Validation::default()` in effect:

| Setting | Value | Implication |
|---|---|---|
| `algorithms` | `[HS256]` | Algorithm-confusion safe |
| `validate_exp` | `true` | Expiry enforced |
| `leeway` | `60` s | Effective validity = TTL + 60 s (≈ 2 min) |
| `required_spec_claims` | `["exp"]` | Tokens missing `exp` rejected |
| `validate_nbf` | `false` | `nbf` **not checked** |
| `validate_aud` / `aud` | `false` / `None` | Audience **not checked** |
| `iss` / `sub` | `None` | Issuer/subject **not checked** |
| `required_private_claims` | `[]` | No custom claim requirements |

**Claims structure** (`jwt-lib/src/model/claims.rs`): `{ "email": String, "exp": i64 }` — the payload contains only email + expiry. No `iss`, `aud`, `nbf`, `iat`, `jti`, `sub`, no roles/scopes.

### 6.4 Token Extraction Weaknesses

```rust
.and_then(|str| str.split(" ").nth(1))
```

- `Bearer` scheme is **never verified** — any header whose 2nd space-separated token is a valid JWT passes (e.g., `Basic <jwt>`).
- Splits on a literal single space — tabs or multiple spaces break extraction (leads to `"No token provided"`).
- No constant-time comparison concerns apply (HMAC is verified by the library), but no rate limiting exists on `/get-token` or `/secret-view`.

### 6.5 Password Handling

**None whatsoever.** `User` contains only `email: String`; there is no password field, no hashing (no bcrypt/argon2/sha2 deps), no credential storage, no user database, no sign-up/login endpoint. The demo is token-issuance-only: any email can obtain a signed token.

### 6.6 Other Security Observations

- Raw `jsonwebtoken` error strings returned in 401/400 bodies (e.g., `ExpiredSignature`, `InvalidSignature`, `MissingRequiredClaim`) — minor internal detail disclosure; prefer generic messages.
- Servers bind to loopback (`localhost`/`127.0.0.1`) — safe by default, but the Docker `EXPOSE`/port mapping contradicts this.
- No `strict_transport_security`, no HTTPS, no CORS config — fine for a demo, worth noting.
- Tokens are short-lived (1 min) with no refresh mechanism, no blacklist/revocation, no `jti`.

---

## 7. CI/CD + Docker

### 7.1 GitHub Actions — `.github/workflows/docker-publish-axum.yml` (the only workflow; exact content)

```yaml
name: Docker

# This workflow uses actions that are not certified by GitHub.
# They are provided by a third-party and are governed by
# separate terms of service, privacy policy, and support
# documentation.

on:
  push:
    branches: ["main"]
    # Publish semver tags as releases.
    tags: ["v*.*.*"]
  pull_request:
    branches: ["main"]

env:
  # Use docker.io for Docker Hub if empty
  REGISTRY: ghcr.io
  # github.repository as <account>/<repo>
  IMAGE_NAME: ${{ github.repository }}

jobs:
  build:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
      # This is used to complete the identity challenge
      # with sigstore/fulcio when running outside of PRs.
      id-token: write

    steps:
      - name: Checkout repository
        uses: actions/checkout@v3

      # Install the cosign tool except on PR
      - name: Install cosign
        if: github.event_name != 'pull_request'
        uses: sigstore/cosign-installer@6e04d228eb30da1757ee4e1dd75a0ec73a653e06 #v3.1.1
        with:
          cosign-release: "v2.1.1"

      # Set up BuildKit Docker container builder
      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@f95db51fddba0c2d1ec667646a06c2ce06100226 # v3.0.0

      # Login against a Docker registry except on PR
      - name: Log into registry ${{ env.REGISTRY }}
        if: github.event_name != 'pull_request'
        uses: docker/login-action@343f7c4344506bcbf9b4de18042ae17996df046d # v3.0.0
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      # Extract metadata (tags, labels) for Docker
      - name: Extract Docker metadata
        id: meta
        uses: docker/metadata-action@96383f45573cb7f253c731d3b3ab81c87ef81934 # v5.0.0
        with:
          images: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}

      # Build and push Docker image with Buildx (don't push on PR)
      - name: Build and push Docker image
        id: build-and-push
        uses: docker/build-push-action@0565240e2d4ab88bba5387d719585280857ece09 # v5.0.0
        with:
          context: .
          file: Dockerfile.axum
          push: ${{ github.event_name != 'pull_request' }}
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

      # Sign the resulting Docker image digest except on PRs.
      - name: Sign the published Docker image
        if: ${{ github.event_name != 'pull_request' }}
        env:
          TAGS: ${{ steps.meta.outputs.tags }}
          DIGEST: ${{ steps.build-and-push.outputs.digest }}
        run: echo "${TAGS}" | xargs -I {} cosign sign --yes {}@${DIGEST}
```

**What the pipeline does:**
1. Triggers on push to `main`, tags `v*.*.*`, and PRs against `main`.
2. Builds **only the axum image** (`file: Dockerfile.axum`, context `.`).
3. Pushes to **GHCR** (`ghcr.io/<owner>/<repo>`) on push/tag (not on PR); tags derived by `docker/metadata-action` (branch tag `main`, semver `v*.*.*`).
4. Uses GitHub Actions cache (`type=gha`, `mode=max`) for BuildKit.
5. Signs the image with **cosign** (sigstore) on push/tag using the OIDC `id-token`.
6. **No test/check/clippy/audit steps** — build & ship only.
7. Actions pinned to SHAs with version comments (checkout@v3, cosign v3.1.1, buildx v3.0.0, login v3.0.0, metadata v5.0.0, build-push v5.0.0).

**Actix workflow (deleted):** `.github/workflows/docker-publish.yml` existed from `a490aeb` to HEAD commit `1dd6df8` (97 lines) — identical template with `file: Dockerfile.actix`. It was removed in the same commit that dropped the actix compose service. So **only axum is published today**.

### 7.2 Dockerfiles

**`Dockerfile.axum` (current, working tree):**

```dockerfile
FROM rust:1.71-slim as build

WORKDIR /app

COPY ./jwt-lib/ ../
COPY ./axum-auth/ .

RUN cargo build --release

FROM rust:1.71-slim

WORKDIR /usr/local/bin

COPY --from=build /app/target/release/axum-auth .

EXPOSE 2323

CMD ["./axum-auth"]
```

- Two-stage build; `rust:1.71-slim` for both builder and runtime (compiler not in final stage).
- Binary staged to `/usr/local/bin/axum-auth`; `EXPOSE 2323`; `CMD ["./axum-auth"]`.
- **Regression (introduced in HEAD commit `1dd6df8`):** `COPY ./jwt-lib/ ../` copies jwt-lib's *contents* into `/` (the parent of `WORKDIR /app`), so `/jwt-lib` never exists and the path dependency `jwt-lib = { path = "../jwt-lib" }` cannot resolve → `cargo build --release` will fail. The pre-`1dd6df8` form was correct: `COPY ./jwt-lib/ ../jwt-lib/`.
- The root workspace manifest (`cargo.toml`) is **never copied** into the image — the container build is a single-package build, not a workspace build (which incidentally masks the lowercase-filename problem).
- No `HEALTHCHECK`, no non-root `USER`, no `--release` cargo profile tuning (e.g. `lto`).

**`Dockerfile.actix` (in git HEAD, deleted from working tree — uncommitted deletion; recovered via `git show HEAD:Dockerfile.actix`):**

```dockerfile
FROM  rust:1.71-slim as build

WORKDIR /app

COPY ./jwt-lib/ ../
COPY ./actix-auth/ .

RUN cargo build --release

FROM rust:1.71-slim

WORKDIR /usr/local/bin

COPY --from=build /app/target/release/actix-auth .

EXPOSE 8080

CMD ["./actix-auth"]
```

Same template, `EXPOSE 8080`, binary `actix-auth`. (Same `../` regression pattern.)

### 7.3 docker-compose.yaml (current)

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

- Single service `axum_auth`, host port 2323 → container 2323.
- **Previous revision** (pre-`1dd6df8`, recovered via `git show`) also had:
  ```yaml
  actix_auth:
    build:
      context: .
      dockerfile: Dockerfile.actix
    ports:
      - "8080:8080"
  ```
  removed in HEAD commit `1dd6df8`, which is consistent with the deleted Dockerfile.actix and the removed actix workflow — the repo is being consolidated onto axum-only containerization.

### 7.4 .dockerignore

```
**/target
```

Only excludes build artifacts — `.git`, `Cargo.lock` etc. would be sent in context unless git-based ignore kicks in.

---

## 8. Summary of Issues & Quirks

### 8.1 Build & Repo Hygiene

1. **Lowercase `cargo.toml`** — the workspace manifest is named `cargo.toml` (all lowercase). Works on Windows/macOS (case-insensitive), but on case-sensitive filesystems (Linux CI, Linux dev, Docker on Linux) Cargo looks for `Cargo.toml` and will not find it. Should be renamed to `Cargo.toml`.
2. **`Cargo.lock` gitignored** — despite the `.gitignore` comment noting lockfiles should be kept for executables. Builds are non-reproducible across clones; the observed resolved versions (§1.4) are only from the local lockfile.
3. **Uncommitted deletion of `Dockerfile.actix`** — present in git HEAD (`23200c3`), deleted in the working tree (`git status` → `D Dockerfile.actix`). If a container build for actix is ever re-enabled, the file must be restored.
4. **Dockerfile `COPY ./jwt-lib/ ../` regression** (both Dockerfiles) — jwt-lib contents land in `/` instead of `/jwt-lib`; the `../jwt-lib` path dependency fails to resolve. Pre-`1dd6df8` used `COPY ./jwt-lib/ ../jwt-lib/`.
5. **Workspace manifest never copied into Docker images** — container builds are standalone-package builds; the workspace-level `cargo.toml` (with its lowercase-name fragility) never participates.
6. **No CI steps for quality** — the workflow only builds/pushes; no `cargo test`, `clippy`, `fmt --check`, or `cargo audit`.

### 8.2 Code & Architecture

7. **"Middleware" is actually an extractor** in both apps (`FromRequestParts` / `FromRequest`) — auth applies only to handlers that declare `Auth` as a parameter. A new protected route could silently be unprotected if the extractor is forgotten. A true global middleware (`middleware::from_fn` / `wrap_fn` / Tower layer) or a route-grouped `Router::layer` would be safer.
8. **`decode_jwt` decodes into `User`, not `Claims`** — `exp` is dropped from the returned data; works only because serde ignores unknown fields. Fragile coupling between `Claims` (encode) and `User` (decode).
9. **Hardcoded secret `"mykey"`, fixed 1-minute TTL, no clock injection** — untestable and insecure for anything beyond a demo (§6).
10. **Naive `split(" ")` header parsing** — Bearer scheme not verified; whitespace edge cases → false 401s.
11. **Raw library error strings leaked to clients** (401/400 bodies).
12. **Inconsistent response envelopes** — axum `/` returns `{"message": ...}` without the `success`/`data` wrapper used everywhere else; actix `/` returns `{"success": true, "data": "Hello World!"}` — no shared contract.
13. **Duplicated code** — both apps re-implement identical controllers/middleware/error shapes; only JWT logic is shared. A shared `auth-http` layer or workspace feature could deduplicate.
14. **Stringly-typed errors** (`Result<_, String>`) — no `thiserror`/`anyhow`/custom error enum; the `unwrap_or_default()` on responses silently swallows builder errors (a 200 with empty body if a response build ever fails).
15. **Idiomatic nits** — `return token;` tail return, `str.split(" ")` without `split_whitespace`, `#[tokio::main]` on a non-`main` function, no `Debug`/`Clone` derives on models/extractors.
16. **Zero test coverage** — nothing protects the JWT round-trip, expiry, or the 401 paths (§5).

### 8.3 Documentation & Process

17. **README is 2 lines** — "Rust API with Docker Containers and JWT Authentication". No run instructions, API docs, or env docs.
18. **Commit convention** — emoji + scope, e.g. `[ :sparkles: ] | setup: server - port (app::actix)`, `[ 👷 ] | create: action - docker-publish > axum (container)`, `[ :passport_control: ] | create: middleware - auth (app::middleware)`. Consistent and greppable.
19. **Consolidation direction** — the last commits progressively removed actix from the container pipeline (workflow deleted, compose service deleted, Dockerfile.actix deleted locally), leaving axum as the published service while the actix crate remains a workspace member.

---

## 9. Git History Summary

### 9.1 Repository Facts

- **41 commits**, all on a single branch `main` (tracking `origin/main`, up to date at time of analysis).
- **Author:** Samuel_Ricardo (commit emails observed: `samuelricardoofficial@gmail.com` and `samuel_ricardo`).
- **License:** MIT, `LICENSE` file: `Copyright (c) 2024 Samuel_Ricardo`.
- **Timeline:** all observable development occurred on a single day — 2024-03-17 (UTC-3). Last commit `1dd6df8` at 15:19 local. No commits, tags, or other branches after that date.
- **No tags** exist; versioning is only via `v*.*.*` tag triggers in the workflow.

### 9.2 Commit Convention

Format: `[ :emoji: ] | <verb>: <subject> (<scope>)` — emoji-categorized conventional commits. Examples (verified):

| Commit | Message |
|---|---|
| `4764f9c` | `[ :heavy_plus_sign: ] | create: lib - jwt (app)` |
| `87c0ceb` | `[ :sparkles: ] | create: fun - get_jwt (lib::jwt)` |
| `a81b5d5` | `[ :heavy_plus_sign: ] | add: lib - jwt (app::axum)` |
| `a5c4df8` | `[ :video_game: ] | create: controller - hello_world (app::controller)` |
| `39fd3e1` | `[ :passport_control: ] | create: middleware - auth (app::middleware)` |
| `9857bb7` | `[ :video_game: ] | create: controller - secret_view_handler (app::controller)` |
| `f47522d` | `[ :vertical_traffic_light: ] | map: route - get_token (app::server)` |
| `23200c3` | `[ :construction_worker: ] | create: Dockerfile - actix (image)` |
| `40b3e0c` / `a123fb2` | `[ 👷 ] | create: action - docker-publish > axum (container)` |
| `a490aeb` | `[ 👷 ] | create: action - docker-publish > actix (container)` |
| `1dd6df8` (HEAD) | `[ :green_heart: ] | fix: container - actix (container)` |

### 9.3 Development Narrative (chronological, from verified commits)

1. `4764f9c` — jwt-lib crate created with `cargo new` boilerplate (`add` fn + `it_works` test).
2. `87c0ceb` — real JWT functions (`get_jwt_secret`/`decode_jwt`) landed; **the only test in history was removed here**.
3. `a81b5d5` — axum-auth crate created, wired to jwt-lib.
4. `a5c4df8` — `hello_world` controller.
5. `3285584` / `1d34d15` — actix server setup (both apps scaffolded in parallel).
6. `90df652` / `276ef9e` — `public_view_handler` + route.
7. `9ec6602` / `39fd3e1` / `9857bb7` — `get_token_handler`, auth middleware, `secret_view_handler`.
8. `f47522d` / `5498cff` / `4a86d1b` — route mappings (`get_token`, `secret_view` ×2).
9. `7ed2162` — actix server port set (8080).
10. `23200c3` — `Dockerfile.actix` created.
11. `a123fb2` → `40b3e0c` → `ec52833` → `fa7e13d` — GHCR `docker-publish` workflow for axum created, updated, fixed.
12. `a490aeb` — actix variant workflow (`docker-publish.yml`) created.
13. `1dd6df8` (HEAD) — **consolidation commit**: deleted the actix workflow, removed the `actix_auth` service from docker-compose.yaml, and changed `Dockerfile.axum`'s `COPY ./jwt-lib/ ../jwt-lib/` → `COPY ./jwt-lib/ ../` (the regression described in §7.2/§8.1).

### 9.4 Working-Tree Divergence from HEAD

- `Dockerfile.actix` — deleted in working tree, still present at HEAD (uncommitted deletion).
- `.github/workflows/docker-publish.yml` (actix) — already removed at HEAD.

### 9.5 History-Only Artifacts (recovered via `git show`, not in working tree)

- `.github/workflows/docker-publish.yml` — 97-line actix workflow, identical template to the axum one with `file: Dockerfile.actix`.
- Pre-`1dd6df8` `docker-compose.yaml` — included the `actix_auth` service (8080:8080).
- Pre-`1dd6df8` `Dockerfile.axum` — correct `COPY ./jwt-lib/ ../jwt-lib/` line.
- Boilerplate `it_works` test in jwt-lib (removed in `87c0ceb`).

---

## Appendix — Key Files Quick Reference (absolute paths)

| Purpose | Path |
|---|---|
| Workspace manifest | `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\cargo.toml` |
| JWT logic | `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\jwt-lib\src\lib.rs` |
| Models | `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\jwt-lib\src\model\user.rs`, `...\model\claims.rs`, `...\model\mod.rs` |
| axum server/routes | `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\axum-auth\src\server.rs` |
| axum auth extractor | `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\axum-auth\src\middleware\auth.rs` |
| axum handlers | `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\axum-auth\src\controller\auth.rs`, `...\controller\mod.rs` |
| actix server/routes | `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\actix-auth\src\server.rs` |
| actix auth extractor | `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\actix-auth\src\middleware\auth.rs` |
| actix handlers | `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\actix-auth\src\controller\auth.rs`, `...\controller\mod.rs` |
| CI workflow | `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\.github\workflows\docker-publish-axum.yml` |
| Docker | `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\Dockerfile.axum`, `...\docker-compose.yaml`, `...\.dockerignore` |
| Analysis report | `C:\Users\Desktop\Desktop\Projects\Rust\rust_jwt\.avanade-method\analysis\project-analysis.md` (this file) |

---

*End of report. All code quotes captured verbatim from the working tree, except `Dockerfile.actix` and pre-`1dd6df8` revisions, which were recovered from git history (`git show`).*