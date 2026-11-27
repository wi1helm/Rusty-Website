# Rusty-NoName Security & Architecture Review

**Last reviewed:** 2026-11-27 · **Review type:** Read-only source/configuration review with local dependency and unit-test checks. This is not a penetration test or production deployment assessment.

## Executive summary

The application has useful foundational controls: Argon2 password hashing, high-entropy server-side sessions, same-origin checks for state-changing requests, security response headers, Askama HTML escaping, parameterized SQL, organization-scoped repository operations, and image/graph input validation. The reserved-route slug check, non-root runtime user, and RustFS digest pin from earlier work are present in this checkout.

The most important remaining risks are missing throttles on public authentication endpoints and the default Compose Redis service having no authentication. There are also session-revocation and profile-update consistency gaps, plus lower-priority abuse, maintainability, and deployment-hardening items. No SQL injection, stored XSS, cross-organization data access, or CSRF bypass was established in this source review; that is not proof of their absence at runtime.

### Current verification snapshot

| Check | Result |
|---|---|
| `cargo test --locked` | 156 passed, 0 failed. Rust emitted non-fatal style/dead-code warnings. |
| `npm run test:js` | 11 passed, 0 failed. |
| `cargo audit --no-fetch` | 323 locked dependencies scanned; 0 known vulnerabilities and 2 unmaintained-crate warnings (`basic-toml`, `chrono`). Uses the local advisory database snapshot; refresh before release. |
| HTTP integration, live deployment, fault injection, browser/accessibility testing | Not run during this review. |

### Priority guide

- **P1 — Address before broad public use:** high-impact exposed weakness or basic abuse control.
- **P2 — Near-term:** meaningful security, session, or data-integrity risk; validate deployment assumptions promptly.
- **P3 — Backlog:** lower-impact bug, defense-in-depth, product-scope question, or maintainability improvement.

Priorities are qualitative, not CVSS scores. “Confirmed” means visible in the reviewed source/configuration, not exploited in a live system.

## Ranked current findings

### P1 — Public login has no throttling (`AUDIT-001`)

- **Severity/status:** High / confirmed.
- **Evidence:** `src/api/login.rs` calls `Authenticator::login` for each valid-format request; no login rate limiter is applied in `src/api/mod.rs` or `src/routes/mod.rs`. Password verification uses Argon2 in `spawn_blocking`.
- **Impact:** Online password guessing and avoidable CPU/worker-pool pressure.
- **Recommendation:** Add distributed limits by source and normalized account identifier, with monitoring and gradual backoff. Keep responses generic and avoid permanent account lockouts that enable denial of service.

### P2 — Public signup has no abuse budget (`AUDIT-002`)

- **Severity/status:** Medium / confirmed.
- **Evidence:** `src/api/creation.rs` accepts public organization creation and delegates to password hashing and durable organization/member/profile writes; no signup throttle or verification gate is wired.
- **Impact:** Automated signup can consume hashing capacity, database connections, and persistent storage.
- **Recommendation:** Apply shared signup limits before expensive work and persistence; add verification or other anti-abuse controls if public registration is intended. Define monitoring and cleanup/retention policy.

### P2 — Compose Redis has no authentication (`AUDIT-014`)

- **Severity/status:** Medium / confirmed for the Compose-defined Redis service; external deployments may differ.
- **Evidence:** `compose.yaml` starts Redis with persistence but no ACL/password configuration. Staging does not publish Redis to the host, but services share the default internal network.
- **Impact:** A compromised or misconfigured container on that network can access Redis. Session IDs are the Redis lookup keys (`session:<id>`) and are also serialized in the stored session object, so read access can expose active bearer credentials. Encrypting only the JSON payload would not hide the key/session ID.
- **Recommendation:** Configure Redis ACLs/authentication, keep Redis on a private data network inaccessible to the proxy, use TLS across trust boundaries, and protect backups. Treat Redis read access as session compromise and document incident-driven session invalidation.

### P2 — No logout or prompt session revocation (`AUDIT-003`)

- **Severity/status:** Medium / confirmed.
- **Evidence:** `SessionManager::delete` exists, but no logout route invokes it and `templates/protected/base.html` has no logout control. Server-side sessions have a 24-hour TTL.
- **Impact:** Users cannot promptly end a session on a shared or lost device; a stolen session remains usable until expiry or manual cache intervention.
- **Recommendation:** Add a same-origin POST logout handler that deletes the server-side session and expires the cookie with matching attributes. Consider a “revoke all sessions” path for password changes and account recovery.

### P2 — Profile fields and links can partially commit (`AUDIT-005`)

- **Severity/status:** Medium data-integrity bug / confirmed from call order; not fault-injected.
- **Evidence:** `src/api/protected/profile.rs` writes profile fields and then links through separate repository operations. `update_profile_links` has a transaction, but it does not include the preceding profile write.
- **Impact:** If the second operation fails, the request reports failure while the public profile contains a mixture of new and old data.
- **Recommendation:** Move the entire profile update into one application use case/repository operation with one database transaction; add a failure-injection test asserting rollback.

### P2 — CSRF origin calculation assumes trusted proxy headers (`AUDIT-018`)

- **Severity/status:** Deployment assumption to validate; no bypass demonstrated.
- **Evidence:** `src/routes/mod.rs` derives the expected scheme from `x-forwarded-proto` and authority from `Host`. `Caddyfile` is the staging edge, but the app does not authenticate which peer supplied forwarded headers.
- **Impact:** Same-origin protection depends on direct app access being blocked and the edge overwriting forwarded headers. A different proxy/topology or exposed backend could violate that assumption.
- **Recommendation:** Restrict the app listener to the trusted proxy/private network, ensure the proxy sanitizes forwarded headers, and consider a configured canonical origin or trusted-proxy policy. Test direct and proxied requests in deployment.

### P3 — Profile link collection has no explicit item cap (`AUDIT-015`)

- **Severity/status:** Low / confirmed missing validation; impact depends on request limits and account access.
- **Evidence:** `UpdateRequest.links` is a `Vec`; validation checks each label and URL but not item count. The repository deletes existing links and inserts each submitted link in a loop. The JSON extractor has a body-size limit, so this is bounded by payload size, not literally unlimited.
- **Impact:** An authenticated account can still submit many valid links, causing numerous rows and sequential database writes.
- **Recommendation:** Set a product-level maximum link count, reject oversized arrays before database work, and test the boundary.

### P3 — Concurrent image replacement can orphan objects (`AUDIT-011`)

- **Severity/status:** Low / plausible race from code order; concurrency was not tested.
- **Evidence:** `src/api/protected/image/image.rs` reads the old key, uploads a fresh UUID object, updates the database, then removes the previously read object. Concurrent uploads can both observe the same old key, leaving the losing new object unreferenced.
- **Impact:** Orphaned S3 objects and avoidable storage cost; no cross-organization impact was identified.
- **Recommendation:** Serialize replacements per organization/image type, or add cautious periodic reconciliation and a concurrent-upload test.

### P3 — Workspace navigation points to missing routes (`AUDIT-006`)

- **Severity/status:** Low UX bug / confirmed.
- **Evidence:** `templates/protected/base.html` links to `/members` and `/settings`; `src/routes/protected/mod.rs` registers only `/dashboard` and `/profile`.
- **Recommendation:** Hide the links until those pages exist, or implement the protected handlers.

### P3 — Signup still permits partial email enumeration (`AUDIT-007`, residual)

- **Severity/status:** Low / mitigated but not eliminated.
- **Evidence:** Duplicate-member signup returns a generic `signup-failed` message, but still returns a conflict response while a new account returns a created response and session. Without verified email, an observer can distinguish those outcomes.
- **Recommendation:** Accept the tradeoff if appropriate; if account privacy is important, use verified email with more uniform external responses. Generic wording alone does not fully resolve enumeration.

### P3 — Recovery path is absent (`AUDIT-008`, product-scope dependent)

- **Severity/status:** Potential product gap; no reset endpoint, page, or login link is registered.
- **Impact:** A user who loses the owner password may permanently lose access.
- **Recommendation:** If required, design single-use expiring reset tokens, verified delivery, generic responses, and rate limits. Revoke sessions after a successful reset.

### P3 — Mutable base service tags reduce deployment reproducibility (`AUDIT-016`)

- **Severity/status:** Low / confirmed configuration concern.
- **Evidence:** Postgres and Redis in `compose.yaml`, and Caddy in `compose.staging.yaml`, use mutable major-version tags. RustFS is digest-pinned; the application runtime is non-root.
- **Recommendation:** Pin production images by digest or an explicitly managed immutable release, then update through a reviewed automated process with vulnerability scanning and rollback notes.

### P3 — Storage provisioning is coupled to application startup (`AUDIT-017`)

- **Severity/status:** Informational least-privilege/design concern; not a demonstrated exploit.
- **Evidence:** `S3ObjectStorage::new` calls `ensure_bucket`; any `head_bucket` error triggers a create attempt. Startup also checks and may upload default images. The adapter handles application object reads, writes, and deletes.
- **Impact:** Runtime credentials may need provisioning permissions that steady-state serving does not need; transient, authorization, and not-found errors are conflated.
- **Recommendation:** Provision bucket/defaults through deployment tooling and give the app only bucket-scoped runtime permissions. Distinguish missing resources from permission/network failures.

## Architecture and future-thinking notes

- **Image upload is a coordination hotspot.** `src/api/protected/image/image.rs` parses multipart data, validates and normalizes images, writes S3 objects, updates PostgreSQL, and compensates on failure. Move orchestration into an image application service with explicit stages and cleanup behavior; keep the handler focused on HTTP translation.
- **Profile update spans repository boundaries.** The handler owns comparison, mapping, and two writes. A profile-update use case that owns the transaction would address `AUDIT-005` and simplify failure tests.
- **Separate infrastructure bootstrap from runtime adapters.** S3 bucket creation/default seeding belongs in deployment or a one-shot bootstrap task, not ordinary request-serving credentials. Keep runtime repository/storage interfaces focused on application operations.
- **Treat Redis as an authentication boundary.** Payload encryption alone does not protect against Redis readers while the bearer ID is present in the key and value. Prioritize ACL/network/backup controls; redesign key storage only if read-only cache exposure is in the threat model.
- **Keep tenant identity server-derived.** Protected handlers obtain the organization from the authenticated session and repository operations bind organization IDs. Preserve this pattern; do not accept client-supplied organization IDs as authorization context.
- **Observability multiplies abuse controls.** Record aggregate rate-limit/authentication events without logging passwords, session IDs, or secret-bearing URLs. Alert on signup/login spikes and Redis availability errors.

## Suggested order of work

1. Add login throttling and signup abuse controls; authenticate and network-restrict production Redis.
2. Implement logout/session revocation and make profile updates atomic; add failure and revocation tests.
3. Validate trusted-proxy assumptions in the actual deployment, cap profile links, and address image replacement cleanup.
4. Remove dead navigation, decide account-recovery/email-verification scope, separate S3 bootstrap permissions, and pin production image digests.
5. Add CI gates for `cargo test --locked`, JavaScript tests, a freshly updated `cargo audit`, and browser-driven auth/profile smoke tests.

## Scope and limitations

- Reviewed Rust routes/auth/config/storage, SQL migrations, Askama templates and JavaScript sinks, dependency manifest/lockfile, Docker/Compose, Caddy, README, and previous report claims. Targeted searches found no direct `innerHTML`, `outerHTML`, `document.write`, or `eval` use in project JavaScript; this is not a complete XSS proof.
- `cargo audit --no-fetch` used the local advisory database snapshot and should be rerun with a refreshed database before release. The scan reported two unmaintained crates, not known vulnerabilities.
- No live HTTP integration run, production/staging deployment review, Docker build, external-service test, browser/accessibility test, load test, or fault/concurrency injection was performed during this review.
- Unit tests do not prove production proxy configuration, Redis ACLs, backup security, or multi-instance behavior.

## Historical appendix — previous audit snapshot (superseded)

The sections below preserve prior audit evidence and IDs for traceability. Their status counts, environment claims, test counts, and verification statements are historical only; use the current findings and verification snapshot above as authoritative.

## Previous critical issues

None identified.

## 3. High-priority issues

### AUDIT-001 — No login throttling

- **Severity:** HIGH; **Status:** CONFIRMED
- **Affected:** `src/api/login.rs`, `login`; `POST /api/auth/login`.
- **Description:** The public handler processes repeated valid-format credential attempts without a rate limit, account/IP throttle, or backoff.
- **Steps:** Repeatedly submit a valid-format email and password to `POST /api/auth/login`; the handler delegates each attempt to password verification without a configured attempt budget.
- **Evidence:** No rate-limiting middleware is attached in `src/api/mod.rs` or `src/routes/mod.rs`; the pre-existing source TODO already identified unlimited attempts.
- **Impact:** Online password guessing and resource exhaustion through repeated password-hash verification.
- **Recommended fix:** Add shared, distributed per-IP/account throttling and monitoring. Avoid revealing whether an account exists or using a lockout policy that enables easy denial of service.

## 4. Medium-priority issues

### AUDIT-002 — Unrestricted organization signup

- **Severity:** MEDIUM; **Status:** CONFIRMED
- **Affected:** `src/api/creation.rs`, `signup`; `POST /api/organizations/create`.
- **Description:** Public signup has no rate limiting or anti-automation control and creates durable organization/member/profile records after password hashing.
- **Steps:** Submit signups with distinct names and email addresses; there is no configured per-IP/signup limit.
- **Evidence:** `signup` has no rate-limit check or challenge; its existing TODO notes automated organization creation.
- **Impact:** Automated signup can consume database storage, connection capacity, and CPU used by password hashing.
- **Recommended fix:** Add shared rate limits and abuse monitoring before expensive hashing and persistence; consider verification appropriate to the product.

### AUDIT-003 — No logout route or session revocation UI

- **Severity:** MEDIUM; **Status:** CONFIRMED
- **Affected:** `src/routes/mod.rs`, `src/routes/protected/mod.rs`, and `templates/protected/base.html`; session lifecycle.
- **Description:** No logout endpoint is registered and the protected navigation exposes no logout control. Server-side sessions remain usable until their 24-hour TTL expires.
- **Steps:** Sign in and inspect the protected navigation or registered API routes; there is no logout action that deletes the session and expires the cookie.
- **Evidence:** API routes contain signup, login, and profile handlers only; `SessionManager` deletion is not invoked by a route. The template navigation has no logout link.
- **Impact:** Users on shared devices cannot promptly revoke an active authenticated session.
- **Recommended fix:** Add a same-origin POST logout handler that deletes the Redis session and clears the cookie with matching security attributes.

### AUDIT-004 — Organization slugs collide with application routes (RESOLVED)

- **Severity:** MEDIUM; **Status:** RESOLVED
- **Affected:** `src/api/creation.rs`, `create_slug`; `src/routes/mod.rs`, `/{slug}` and static routes.
- **Original finding:** Slug generation accepted names such as `Login` and `Create`, yielding slugs that collide with static routes.
- **Original reproduction:** In the isolated integration environment, signup with the organization name `Login` succeeded and persisted slug `login`, while `GET /login` served the login form.
- **Resolution:** Signup rejects slugs that match registered public/protected routes or top-level API, image, and asset paths before password hashing or persistence. Reservation derives from the same path constants used by route registration.
- **Verification:** Unit tests cover all registered root route names and a valid non-reserved slug; the signup integration scenario asserts that `Login` redirects with `invalid-slug`.

### AUDIT-005 — Profile fields and links are not updated atomically

- **Severity:** MEDIUM; **Status:** CONFIRMED
- **Affected:** `src/api/protected/profile.rs`, `update`; `POST /api/profile/update`.
- **Description:** The handler commits profile fields with `update_profile`, then persists links separately with `update_profile_links`. The latter method has its own transaction, but that transaction does not include the earlier profile write.
- **Steps:** If the link write fails after the profile write succeeds, the handler returns an error while the new display name/about and old links remain persisted. This failure sequence was verified from the call order, not injected during runtime tests.
- **Evidence:** The handler performs two sequential repository calls and has an existing TODO documenting the non-atomic operation.
- **Impact:** A failed update can leave partially changed public profile data and mislead users who retry after an error.
- **Recommended fix:** Provide one repository operation that updates profile fields and links inside the same database transaction.

## 5. Low-priority issues

### AUDIT-006 — Protected navigation links to missing pages

- **Severity:** LOW; **Status:** CONFIRMED
- **Affected:** `templates/protected/base.html`; `/members` and `/settings`.
- **Description:** The protected header renders Members and Settings links, but `src/routes/protected/mod.rs` registers only `/dashboard` and `/profile`.
- **Steps:** Select Members or Settings from the protected navigation; no corresponding protected handler is registered, so the request falls through to the public slug route/not-found behavior.
- **Evidence:** Template anchors are present; protected router has no matching routes.
- **Impact:** Users encounter dead navigation or a public-profile lookup instead of the expected workspace page.
- **Recommended fix:** Implement the protected pages or hide/remove these links until the pages exist.

### AUDIT-007 — Signup reveals whether an email is registered (RESOLVED)

- **Severity:** LOW; **Status:** RESOLVED
- **Affected:** `src/api/creation.rs`, `signup`; `POST /api/organizations/create`.
- **Original finding:** Duplicate emails received a distinct `email-taken` redirect and explicit “account already exists” message.
- **Original reproduction:** A duplicate signup in the isolated test environment returned `/create?error=email-taken`.
- **Resolution:** Duplicate-email errors now use a generic signup failure and client message rather than explicitly confirming account membership; users who already have access are directed to log in. The successful-signup path still differs, so fully uniform responses would require an email-verification flow, which the application does not currently provide.
- **Verification:** A unit test checks the generic redirect code; the signup integration scenario asserts the duplicate-email response uses `signup-failed`.

### AUDIT-008 — No password-recovery flow

- **Severity:** LOW; **Status:** POTENTIAL (product scope not documented)
- **Affected:** `src/api/mod.rs`, `src/routes/public/mod.rs`, and `templates/public/login.html`.
- **Description:** No password-reset endpoint/page or recovery link is registered or rendered.
- **Steps:** Open the login page and inspect the public API/page routes; no recovery flow is available.
- **Evidence:** Public API routes register organization creation and login only; public page routes register login, create, organization, and profile pages.
- **Impact:** If the application is intended for real users, a forgotten owner password can permanently block organization access.
- **Recommended fix:** If recovery is in scope, add verified, single-use, expiring reset tokens, safe delivery, and rate limits.

### AUDIT-009 — README contains stale security and migration claims (RESOLVED)

- **Severity:** LOW; **Status:** RESOLVED
- **Affected:** `README.md`, “Known limitations and follow-ups”.
- **Original finding:** README incorrectly described plaintext password storage and seven SQLx migrations.
- **Resolution:** README now documents Argon2 password hashing, legacy credential migration concerns, and the current migration count of eight.
- **Verification:** Compared the README with signup/login password handling and the SQL migration files.

### AUDIT-010 — Runtime container runs as root (RESOLVED)

- **Severity:** LOW; **Status:** RESOLVED
- **Affected:** `Dockerfile`, final runtime stage.
- **Original finding:** The final runtime stage had no `USER` directive and started the application as root.
- **Resolution:** The image now runs the app as numeric UID/GID `10001:10001`; its binary and static assets are read-only runtime inputs.
- **Verification:** `docker build --tag rusty-noname:docker-audit-check .` succeeded; image metadata and a runtime shell check confirmed UID/GID `10001:10001`, executable app binary, and readable static assets.

### AUDIT-011 — Concurrent image uploads can orphan storage objects

- **Severity:** LOW; **Status:** POTENTIAL
- **Affected:** `src/api/protected/image/image.rs`, `upload`; `POST /api/profile/image`.
- **Description:** Each upload reads the current image key, writes a new UUID-keyed object, updates the database, and deletes only the previously read key. Two overlapping uploads can both read the same old key; the last database update wins, leaving the other new object unreferenced.
- **Steps:** Run two uploads for the same organization/image type concurrently and inspect object storage for an object not referenced by `organization_profile_images`. This race was not injected in the runtime test.
- **Evidence:** The old key is read before the object write and profile update; there is no per-image lock or compare-and-swap around replacement.
- **Impact:** Repeated concurrent updates can accumulate orphaned image objects and storage costs.
- **Recommended fix:** Serialize replacements per organization/image type or periodically reconcile unreferenced objects safely.

### AUDIT-012 — Vulnerable transitive `h2 0.3.27` dependency (RESOLVED)

- **Severity:** LOW; **Status:** RESOLVED by dependency remediation
- **Affected:** `Cargo.lock`; `src/storage/object/s3.rs`; AWS S3 HTTP client dependency tree.
- **Original finding:** `aws-sdk-s3 1.145.0` enabled `h2 0.3.27` through the legacy Smithy/Hyper 0.14 transport; RustSec `RUSTSEC-2026-0258` affects that `h2` line.
- **Resolution:** Updated to `aws-sdk-s3 1.161.0`, disabled default features, and selected the current single-region HTTPS client features; the obsolete `rustls` feature that enabled the legacy transport is no longer selected. The graph now contains only patched `h2 0.4.19`.
- **Verification:** `cargo tree --locked -i h2@0.4.19` confirms the new AWS Smithy/Hyper 1 path; `cargo audit --no-fetch` reports no vulnerability. S3 image upload and retrieval integration checks pass.

## 6. Informational findings

### AUDIT-013 — Mutable RustFS image tag (RESOLVED)

- **Severity:** INFO; **Status:** RESOLVED
- **Affected:** `compose.yaml`, `storage` service.
- **Original finding:** The service used the mutable `rustfs/rustfs:latest` tag.
- **Resolution:** Pinned RustFS `1.0.6` to its multi-platform OCI index digest `sha256:ee717072290bec9b11abfc0365f8310b00df1cd9b3639943edae47bdef05044a`.
- **Verification:** `docker buildx imagetools inspect rustfs/rustfs:1.0.6` confirmed the pinned index digest and `linux/amd64`/`linux/arm64` manifests; `docker compose config --images` resolved the exact pinned reference. Base, dev, staging, and integration Compose configurations validated. RustFS uses its documented non-root UID `10001`; the Compose service uses a named data volume.

## 7. Security findings

- `AUDIT-001`: online credential guessing and password-verification resource pressure.
- `AUDIT-002`: automated signup can consume persistent database and CPU resources.
- `AUDIT-003`: users cannot promptly revoke sessions from shared devices.
- `AUDIT-007`: resolved; duplicate signup no longer explicitly confirms whether an email is registered.
- `AUDIT-010`: resolved; the application image now runs as UID/GID `10001:10001`.
- `AUDIT-012`: the previously vulnerable transitive HTTP/2 client dependency is resolved; see the dependency matrix below.
- No SQL injection, stored XSS, cross-organization IDOR, or CSRF bypass was confirmed in the reviewed paths. The project uses SQLx bind parameters, Askama escaping, organization-scoped repository queries, and same-origin checks; integration coverage confirmed protected API redirects and input rejection.

## 8. Performance findings

- `AUDIT-001` and `AUDIT-002` create avoidable CPU/database pressure because expensive authentication/signup work is not throttled.
- `AUDIT-011` can leave unreferenced S3 objects after overlapping image replacements.
- No N+1 query or missing pagination defect was confirmed in the examined profile/graph data paths. A full production-scale benchmark was not run.

## 9. UI/UX findings

- `AUDIT-003`: no logout affordance.
- `AUDIT-006`: Members and Settings navigation targets have no protected page handlers.
- `AUDIT-008`: no password-recovery UI or endpoint (scope needs product confirmation).
- Interactive browser, keyboard-only, screen-reader, responsive-layout, and visual-contrast checks were not available; no additional visual/accessibility defect is claimed as confirmed.

## 10. Dependency vulnerabilities and advisories

### Resolved RustSec vulnerability findings (18 original entries)

The IDs and affected versions below are from an audit of the starting committed lockfile. Every entry is resolved in the current lockfile; current `cargo audit --no-fetch` reports zero vulnerabilities. “Removed” means the affected package version is no longer present in the dependency graph or lockfile.

| Advisory ID | Affected dependency: previous → current | Parent dependency / change | Status |
|---|---|---|---|
| `RUSTSEC-2026-0594` | `cookie 0.18.2` → `0.18.3` | `axum-extra 0.12.6`; compatible patched cookie version selected in the lockfile. | RESOLVED |
| `RUSTSEC-2026-0387` | `crypto-bigint 0.5.5` → removed | AWS SDK SigV4a feature; disabled unused SDK defaults and retained single-region SigV4 only. | RESOLVED |
| `RUSTSEC-2026-0385` | `ctutils 0.4.2` → `0.4.8` | `digest 0.11` chain used by AWS SigV4 and SQLx; compatible patched release selected. | RESOLVED |
| `RUSTSEC-2026-0258` | `h2 0.3.27` → removed; `h2 0.4.19` remains | AWS SDK 1.145.0 legacy Smithy/Hyper 0.14 client; upgraded to 1.161.0 and omitted the legacy `rustls` feature. | RESOLVED |
| `RUSTSEC-2026-0459` | `libsqlite3-sys 0.37.0` → `0.39.1` | SQLx optional SQLite backend; upgraded SQLx `0.9.0` → `0.9.1`. SQLite is not in the active app dependency tree, but the lockfile package is now patched. | RESOLVED |
| `RUSTSEC-2026-0349` | `rustls 0.21.12` → removed | AWS SDK legacy HTTP client; removed with the legacy Hyper 0.14 transport. | RESOLVED |
| `RUSTSEC-2026-0346` | `rustls 0.21.12` → removed | AWS SDK legacy HTTP client; removed with the legacy Hyper 0.14 transport. | RESOLVED |
| `RUSTSEC-2026-0349` | `rustls 0.23.44` → `0.23.46` | Current AWS HTTPS client; patched compatible Rustls release selected. | RESOLVED |
| `RUSTSEC-2026-0348` | `rustls 0.23.44` → `0.23.46` | Current AWS HTTPS client; patched compatible Rustls release selected. | RESOLVED |
| `RUSTSEC-2026-0347` | `rustls 0.23.44` → `0.23.46` | Current AWS HTTPS client; patched compatible Rustls release selected. | RESOLVED |
| `RUSTSEC-2026-0285` | `rustls 0.23.44` → `0.23.46` | Current AWS HTTPS client; patched compatible Rustls release selected. | RESOLVED |
| `RUSTSEC-2026-0346` | `rustls 0.23.44` → `0.23.46` | Current AWS HTTPS client; patched compatible Rustls release selected. | RESOLVED |
| `RUSTSEC-2026-0104` | `rustls-webpki 0.101.7` → removed | Rustls 0.21 legacy client; removed with that client. | RESOLVED |
| `RUSTSEC-2026-0382` | `rustls-webpki 0.101.7` → removed | Rustls 0.21 legacy client; removed with that client. | RESOLVED |
| `RUSTSEC-2026-0098` | `rustls-webpki 0.101.7` → removed | Rustls 0.21 legacy client; removed with that client. | RESOLVED |
| `RUSTSEC-2026-0099` | `rustls-webpki 0.101.7` → removed | Rustls 0.21 legacy client; removed with that client. | RESOLVED |
| `RUSTSEC-2026-0382` | `rustls-webpki 0.103.15` → `0.103.16` | Rustls 0.23 current client; patched compatible release selected. | RESOLVED |
| `RUSTSEC-2026-0592` | `url 2.5.8` → `2.5.9` | Direct dependency; exact pin updated to the patched release. | RESOLVED |

### Unmaintained advisories still present

| Advisory ID | Dependency | Parent and evaluation | Status |
|---|---|---|---|
| `RUSTSEC-2026-0374` | `basic-toml 0.1.10` | `askama_derive` via Askama; no fixed upgrade is available. Keep Askama’s current configuration behavior and monitor for an upstream parser replacement. | UNRESOLVED — unmaintained, no known vulnerability |
| `RUSTSEC-2026-0543` | `chrono 0.4.45` | Used directly for PostgreSQL `DateTime<Utc>` data models and through SQLx’s `chrono` feature; no fixed upgrade is available. `time` plus SQLx’s `time` feature is an alternative, but replacing the timestamp types would require a broader data-model/mapping migration, so it was not made here. | UNRESOLVED — unmaintained, no known vulnerability |

### Other original audit warnings resolved

| Warning | Previous → current | Parent / reason and change | Status |
|---|---|---|---|
| `RUSTSEC-2026-0386` (`tokio` unsoundness) | `tokio 1.53.1` → `1.54.1` | Direct runtime dependency; updated beyond the advisory’s patched minimum `1.53.3`. | RESOLVED |
| `RUSTSEC-2024-0436` (`paste` unmaintained) | `paste 1.0.15` → removed | Pulled by unused EXR/AVIF codec features in `image`; disabled image defaults and enabled only JPEG, PNG, and WebP, which are the formats accepted by `ImageValidator`. | RESOLVED |
| `RUSTSEC-2026-0441` (`pin-utils` unmaintained) | `pin-utils 0.1.0` → removed | Transitive dependency of `aws-smithy-http 0.64.0` under `aws-sdk-s3 1.145.0`; the AWS SDK update removes it from the resolved lockfile. | RESOLVED |
| Yanked crate (no RustSec advisory ID) | `chacha20 0.10.1` → `0.10.2` | Selected through `rand 0.10.2`; the upstream SSE2 backend used an SSE4.1 intrinsic, producing undefined behavior on unsupported CPUs. Upstream issue [#579](https://github.com/RustCrypto/stream-ciphers/issues/579) and the 0.10.2 change record the fix. | RESOLVED |

The current audit therefore has **zero vulnerabilities and two allowed warnings** (`basic-toml` and `chrono`). No advisories were ignored or suppressed. The separately checked `RUSTSEC-2024-0363` does not apply to SQLx `0.9.1` (its affected range ends at `0.8.0`).

Removing unused AWS SigV4a also exposed that Argon2 salt generation had relied on unrelated feature unification for `OsRng`. `rand_core 0.6.4` with `getrandom` is now explicit, and the password module imports `OsRng` directly; hashing behavior is unchanged and its tests pass.

## 11. Previously fixed issues and possible regressions

- Current signup/login code uses password hashing and verification; the README plaintext-password warning is stale rather than a reintroduced vulnerability.
- Recent graph/image paths passed the supplied integration scenarios for persistence, invalid graph rejection, valid image upload, normalized WebP retrieval, malformed multipart, unsupported content type, and oversized uploads.
- No additional regression was confirmed from the inspected recent history. Full history archaeology and staging/production comparison were not performed.

## 12. Areas that could not be tested

- Browser-only visual, responsive, keyboard, screen-reader, and client-side interaction flows: no browser tool was available.
- JavaScript tests and the normal `npm test` entry point: `npm` is unavailable. The integration scenarios were run directly, bypassing their `npm test` precheck; Rust tests were run separately.
- Partial database failure during profile updates, simultaneous image uploads, Redis/database outages, and external S3 peer behavior: not fault-injected.
- Production/staging deployment, external identity/email integration, and real-user workflows were not available.

## 13. Recommended testing improvements

- Add CI checks for JavaScript tests, `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo audit` against the committed lockfile.
- Add browser-driven tests for all navigation links, login/signup/recovery/logout, form validation, keyboard focus, responsive layouts, and accessible names.
- Add failure-injection tests proving profile fields and links roll back together, plus concurrent image-upload tests that detect orphan cleanup behavior.
- Add regression tests for reserved slugs, login/signup throttling, duplicate-email response policy, and session revocation.
- `cargo clippy --locked --all-targets -- -D warnings` fails on 12 existing dead-code and style lints; plain `cargo clippy --locked --all-targets` completes with warnings. Address these independently rather than suppressing them.