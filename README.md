# rust-axum-api

Simple authentication API built with Rust and Axum.

## Goal

This project provides a minimal backend to:
- register users in PostgreSQL (`/auth/register`)
- log in users by validating credentials in PostgreSQL (`/auth/login`)
- issue standard JWT tokens (`/auth/login`)
- log out users by revoking JWT tokens in Redis (`/auth/logout`)

## Verified features

- ✅ Global HTTP error handling:
  - `404` for unknown routes
  - `408` for timeout middleware failures
  - `413` for body-size limit rejections
  - `500` fallback for unhandled internal failures
- ✅ Unified response wrapper:
  - all handler responses use a shared envelope (`status`, `code`, `data`, `messages`, `date`)
- ✅ Graceful shutdown:
  - handles `Ctrl+C` and `SIGTERM` and closes PostgreSQL/Redis resources cleanly
- ✅ Hot reload friendly runtime:
  - supports `listenfd` socket handoff, so you can use `systemfd + cargo-watch` in dev
- ✅ Strict startup configuration validation:
  - required env vars are validated at boot with actionable error messages
- ✅ Standard request input validation at the web adapter boundary:
  - reusable `ValidatedJson<T>` extractor validates DTOs before use case execution
  - email format and required fields are enforced with `validator`

## What it does

- Exposes JSON HTTP endpoints for register and login.
- Hashes passwords with `bcrypt` before saving.
- Validates credentials against the `users` table in PostgreSQL.
- Issues signed JWT tokens on successful login.
- Stores JWT revocations in Redis for logout semantics.
- Standardizes HTTP responses with a shared response wrapper.

## Architecture

Layered hexagonal structure:

- `src/domain/`: domain rules and ports (traits).
- `src/application/`: use cases (`AuthUseCase`).
- `src/infrastructure/`: concrete adapters (web, postgres, redis, wiring).
- `src/database/`: connection services and SQL schema.
- `src/config/`: environment loading and validation.

## Endpoints

### `POST /auth/register`

Request body:

```json
{
  "email": "user@example.com",
  "password": "supersecret"
}
```

Successful response:

```json
{
  "status": "CREATED",
  "code": 201,
  "data": {
    "user_id": "UUID"
  },
  "messages": ["User registered successfully"],
  "date": "2026-01-01T00:00:00+00:00"
}
```

### `POST /auth/login`

Request body:

```json
{
  "email": "user@example.com",
  "password": "supersecret"
}
```

Successful response:

```json
{
  "status": "OK",
  "code": 200,
  "data": {
    "token": "<jwt-token>"
  },
  "messages": ["Login successful"],
  "date": "2026-01-01T00:00:00+00:00"
}
```

### `POST /auth/logout`

Request body:

```json
{
  "token": "<jwt-token>"
}
```

Successful response:

```json
{
  "status": "OK",
  "code": 200,
  "data": {
    "logout": true
  },
  "messages": ["Logout successful"],
  "date": "2026-01-01T00:00:00+00:00"
}
```

## Data model (PostgreSQL)

The project initializes the base schema at startup.

`users` table:
- `id UUID PRIMARY KEY`
- `email VARCHAR UNIQUE NOT NULL`
- `password_hash VARCHAR NOT NULL`
- `created_at TIMESTAMPTZ`
- `updated_at TIMESTAMPTZ` (with trigger)

## JWT + Redis revocation

On successful login:
- the API returns a signed JWT (`HS256`)
- token includes standard claims (`sub`, `iat`, `exp`, `jti`)

On logout:
- token is validated first
- Redis stores a revocation key `revoked_jwt:<token>` with TTL until token expiry

## Required environment variables

```env
ENVIRONMENT=development
HOST=127.0.0.1
PORT=3000
PROTOCOL=http
MAX_REQUEST_BODY_SIZE=1048576
DEFAULT_TIMEOUT_SECONDS=30
DB_HOST=localhost
DB_PORT=5433
DB_NAME=rust_axum_api
DB_USER=postgres
DB_PASSWORD=postgres
JWT_SECRET=change-me-with-a-long-random-secret
JWT_EXP_SECONDS=86400
REDIS_URL=redis://127.0.0.1:6379
```

## Run locally

### Prerequisites

- Rust toolchain installed (`rustup`, `cargo`)
- Docker + Docker Compose installed

### 1) Create local environment file

Create `.env` at project root (or reuse the existing one) with at least:

```env
ENVIRONMENT=development
HOST=127.0.0.1
PORT=3000
PROTOCOL=http
MAX_REQUEST_BODY_SIZE=1048576
DEFAULT_TIMEOUT_SECONDS=30
DB_HOST=localhost
DB_PORT=5433
DB_NAME=rust_axum_api
DB_USER=postgres
DB_PASSWORD=postgres
JWT_SECRET=change-me-with-a-long-random-secret
JWT_EXP_SECONDS=86400
REDIS_URL=redis://127.0.0.1:6379
```

### 2) Start PostgreSQL and Redis

From project root:

```bash
docker compose --env-file .env -f docker/docker-compose.dev.yml up -d
```

### 3) Run the API

```bash
cargo run
```

The API starts on `http://127.0.0.1:3000` by default.
On startup it initializes the base PostgreSQL schema automatically.

## Quick verification

```bash
curl -X POST http://127.0.0.1:3000/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"user@example.com","password":"supersecret"}'

curl -X POST http://127.0.0.1:3000/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"user@example.com","password":"supersecret"}'
```

## Manual feature verification

Run these after starting the API locally.

### Verify 404 (unknown route)

```bash
curl -i http://127.0.0.1:3000/not-found
```

Expected: HTTP `404`.

### Verify 413 (body too large)

```bash
python - <<'PY'
import requests
payload = "x" * (1024 * 1024 + 1024)  # slightly above 1MB default
resp = requests.post(
    "http://127.0.0.1:3000/auth/register",
    json={"email": "big@example.com", "password": payload},
)
print(resp.status_code)
print(resp.text[:300])
PY
```

Expected: HTTP `413`.

### Verify 408 (timeout middleware)

Set a low timeout in `.env` (for example):

```env
DEFAULT_TIMEOUT_SECONDS=1
```

Restart the API and call the debug sleep endpoint:

```bash
curl -i http://127.0.0.1:3000/debug/sleep/3
```

Expected: HTTP `408`.

### Verify register flow

```bash
curl -i -X POST http://127.0.0.1:3000/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"feature-check@example.com","password":"supersecret"}'
```

Expected: HTTP `201`, JSON envelope with `data.user_id`.

### Verify login flow (JWT issuance)

```bash
curl -i -X POST http://127.0.0.1:3000/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"feature-check@example.com","password":"supersecret"}'
```

Expected: HTTP `200`, JSON envelope with `data.token`.

### Verify logout flow + Redis revocation storage

Use the JWT returned by login:

```bash
curl -i -X POST http://127.0.0.1:3000/auth/logout \
  -H "Content-Type: application/json" \
  -d '{"token":"<jwt-token>"}'
```

Expected: HTTP `200`, JSON envelope with `data.logout = true`.

Then verify revocation key exists in Redis:

```bash
redis-cli KEYS "revoked_jwt:*"
```

Expected: at least one `revoked_jwt:<token>` key.

## Current scope

This backend is intentionally focused on basic authentication with Postgres + Redis.
