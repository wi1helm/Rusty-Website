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
| `RUST_LOG` | Optional tracing filter; defaults to `info` |

Browsers do not connect to object storage directly. Profile HTML uses root-relative `/images/{key}` URLs, served through the app's image proxy, so image requests stay on the same host, port, and scheme as the page. `S3_ENDPOINT` is only used for server-to-storage traffic; configure it to the address reachable from the app process (for example, `http://localhost:9000` for a host-run development app or `http://storage:9000` inside Compose).

The Docker Compose infrastructure additionally needs `POSTGRES_USER`, `POSTGRES_PASSWORD`, and `POSTGRES_DB`. `POSTGRES_PORT` and `REDIS_PORT` optionally change the development host ports (defaults `5432` and `6379`). Keep credentials in local environment files or a secret manager; do not commit them.

## Local development

The development script expects a local `.env.dev` file, `cargo-watch`, and Node.js/npm at the version pinned in `.nvmrc`. Copy the tracked example and adjust its values for your setup; `.env.dev` is git-ignored:

```sh
cp .env.example .env.dev
./scripts/dev.sh up
```

This starts PostgreSQL, Redis, and object storage with Docker Compose, then uses `cargo watch` to run `npm test` (JavaScript and Rust unit tests) before starting the Rust app. The tests rerun after each watched file change; integration tests are not part of this loop. Stop the app with `Ctrl+C`; infrastructure helpers are also available:

If startup reports that the HTTP bind address is already in use, set `APP_PORT` in `.env.dev` to an unused port and retry. The development script checks for an existing TCP listener before starting the services.

```sh
./scripts/dev.sh status
./scripts/dev.sh logs
./scripts/dev.sh down
```

The repository pins its Node.js major version in `.nvmrc`. With `nvm`, select it from the project root using `nvm use` (or install that Node.js version another way). Run both the JavaScript unit tests and Rust test suite with:

```sh
npm test
```

For a quick frontend-only check, run `npm run test:js`.

Run the integration tests with:

```sh
./scripts/integration-test.sh
```

This requires Docker Compose, Node.js/npm at the version in `.nvmrc`, the Rust toolchain, `curl`, and `python3`. The script runs `npm test` (JavaScript and Rust tests), starts PostgreSQL, Redis, and S3-compatible storage in a uniquely named Compose project with temporary host ports, builds and starts the app, and runs all scenarios against those services. Scenarios cover signup and sessions, login and access control (including expired sessions), profile and graph persistence, graph validation, and image uploads with invalid-input cases.

Run just one scenario by name when iterating:

```sh
./scripts/integration-test.sh signup-session
./scripts/integration-test.sh login-access
./scripts/integration-test.sh profile-update
./scripts/integration-test.sh graph-update
./scripts/integration-test.sh image-upload
```

The script removes only that project's containers and volumes on exit; it does not use or modify the development or staging Compose projects.

`./scripts/sqlx-prepare.sh` refreshes SQLx offline metadata after query or migration changes. It resets the development database first, so do not use it with data you need to keep. The staging helper uses `.env.staging`, which must be created locally. Its `up`, `rebuild`, and `reset` actions run `npm test` first and stop if either test suite fails:

```sh
./scripts/staging.sh up
```

## Known limitations and follow-ups

- **Legacy credentials:** Passwords are now hashed with Argon2 and verified against the stored hash. Any accounts created by an earlier plaintext-password implementation will need a password reset or a safe migration before those users can log in.
- **Session logout:** There is no logout route, although the session manager can delete a session. Sessions currently expire after 24 hours; add a route that deletes the server-side session and clears the browser cookie.
- **Database rollback:** The eight current SQLx migrations have no corresponding down migrations, so `sqlx migrate revert` cannot roll them back.
- **Developer onboarding:** `.env.example` provides placeholder settings; copy it to a local `.env.dev` or `.env.staging` and replace values as needed. Local environment files remain ignored so real credentials are not committed.
- **API response shape:** Several API handlers redirect to HTML pages on errors instead of returning structured HTTP/JSON errors, coupling API behavior to browser navigation.
