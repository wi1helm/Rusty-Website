# Rusty-NoName
*This documentation is AI generated*

Rusty-NoName is an Axum web application. It renders HTML pages with Askama and uses PostgreSQL, Redis-backed sessions, and S3-compatible object storage.

## Architecture and request flow

At startup, `src/main.rs` loads and validates environment configuration, initializes tracing, and asks `src/app.rs` to construct the application. The app connects to PostgreSQL and runs pending SQLx migrations, connects to Redis and object storage, builds shared handler state, and binds the HTTP listener.

The root router in `src/routes/mod.rs` combines page routes, API routes, and static assets. It also applies CSRF checks to state-changing requests and adds security headers. Protected page and API routers apply authentication middleware: the session cookie identifies a server-side session in Redis, then PostgreSQL resolves the member and organization before the handler runs.

Handlers use repository traits rather than depending directly on PostgreSQL. `ProfileService` combines organization, profile, position, and image data for profile views. Redis and S3 implementations are similarly behind storage interfaces.

### Source map

| Path | Responsibility |
| --- | --- |
| `src/main.rs` | Process entry point and shared Axum `Context` |
| `src/app.rs` | Infrastructure initialization, dependency wiring, and server startup |
| `src/config.rs` | Environment parsing, validation, and cookie policy |
| `src/routes/` | HTML page routes, static files, CSRF checks, and response headers |
| `src/api/` | Form/API handlers and request validation |
| `src/auth/` | Credential authentication, session lifecycle, and protected-request middleware |
| `src/services/` | Application-level composition of repository and storage operations |
| `src/storage/database/` | Database models, repository interfaces, mappings, and PostgreSQL implementation |
| `src/storage/cache/` | Cache interface and Redis implementation used for sessions |
| `src/storage/object/` | Object-storage interface, S3 implementation, and default images |
| `migrations/` | SQLx schema migrations |
| `templates/`, `static/` | Askama templates and browser assets |

### Main routes

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/login`, `/create`, `/organization` | Public HTML pages |
| `GET` | `/{slug}` | Public organization profile |
| `GET` | `/dashboard`, `/profile` | Authenticated pages |
| `POST` | `/api/organizations/create` | Organization signup |
| `POST` | `/api/auth/login` | Member login |
| `POST` | `/api/profile/update` | Update the authenticated organization's profile |
| `POST` | `/api/profile/image` | Upload a profile image |
| `GET` | `/images/{*key}` | Retrieve an image by storage key, including nested keys |

## Configuration

`src/config.rs` requires the following application variables. `APP_ENV` accepts `development`/`dev`, `staging`, or `production`/`prod`; `APP_HOST` must be an IP address and `APP_PORT` must be a non-zero port. `RUST_LOG` is optional and defaults to `info`.

| Variable | Purpose |
| --- | --- |
| `APP_ENV` | Environment name; also controls the session cookie's `Secure` setting |
| `APP_HOST`, `APP_PORT` | IP address and port for the HTTP listener |
| `DATABASE_URL` | PostgreSQL connection URL |
| `CACHE_URL` | Redis connection URL |
| `S3_ENDPOINT` | S3-compatible API endpoint |
| `S3_BUCKET` | Bucket used for application objects |
| `S3_ACCESS_KEY`, `S3_SECRET_KEY` | Object-storage credentials |
| `S3_PUBLIC_URL` | Base URL used to form browser-facing object URLs |
| `RUST_LOG` | Optional tracing filter; defaults to `info` |

The Docker Compose infrastructure additionally needs `POSTGRES_USER`, `POSTGRES_PASSWORD`, and `POSTGRES_DB`. `POSTGRES_PORT` and `REDIS_PORT` optionally change the development host ports (defaults `5432` and `6379`). Keep credentials in local environment files or a secret manager; do not commit them.

## Local development

The development script expects a local `.env.dev` file and `cargo-watch` to be installed. Copy the tracked example and adjust its values for your setup; `.env.dev` is git-ignored:

```sh
cp .env.example .env.dev
./scripts/dev.sh up
```

This starts PostgreSQL, Redis, and object storage with Docker Compose, then runs the Rust app with `cargo watch`. Stop the app with `Ctrl+C`; infrastructure helpers are also available:

```sh
./scripts/dev.sh status
./scripts/dev.sh logs
./scripts/dev.sh down
```

Run the Rust test suite with:

```sh
cargo test
```

`./scripts/sqlx-prepare.sh` refreshes SQLx offline metadata after query or migration changes. It resets the development database first, so do not use it with data you need to keep. The staging helper uses `.env.staging`, which must be created locally:

```sh
./scripts/staging.sh up
```

## Known limitations and follow-ups

- **Critical — password storage:** Signup currently passes the submitted password unchanged into the database field named `password_hash`, and login compares the submitted password directly. Do not deploy this authentication flow for real users until passwords are stored using a dedicated password-hashing algorithm and verified against hashes; existing credentials will also need a migration/reset plan.
- **Session logout:** There is no logout route, although the session manager can delete a session. Sessions currently expire after 24 hours; add a route that deletes the server-side session and clears the browser cookie.
- **Organization URL collisions:** Public profiles use the catch-all `/{slug}` route. Reserve application route names for organization slugs so new pages and API paths cannot be shadowed by organizations.
- **Database rollback:** The seven current SQLx migrations have no corresponding down migrations, so `sqlx migrate revert` cannot roll them back.
- **Developer onboarding:** `.env.example` provides placeholder settings; copy it to a local `.env.dev` or `.env.staging` and replace values as needed. Local environment files remain ignored so real credentials are not committed.
- **API response shape:** Several API handlers redirect to HTML pages on errors instead of returning structured HTTP/JSON errors, coupling API behavior to browser navigation.
