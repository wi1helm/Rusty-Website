FROM rust:1.100.0-trixie AS builder

WORKDIR /app

ARG SQLX_OFFLINE=true
ENV SQLX_OFFLINE=${SQLX_OFFLINE}

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY static ./static
COPY templates ./templates
COPY migrations ./migrations
COPY .sqlx ./.sqlx

# TODO: Remove tests from the Dockerfile. Running tests during image build
# couples CI/CD to the build process and means every `docker build` runs the
# full test suite, slowing down builds. Tests should run in a dedicated CI
# pipeline step, not during container builds. The Dockerfile should only
# build the binary. If you want to ensure tests pass before building, run
# them as a separate step in your CI/CD workflow (e.g. GitHub Actions).
# Search terms: "Docker multi-stage best practices", "CI/CD pipeline Rust",
# "separate tests from build Dockerfile".
# RUN cargo test --release

RUN cargo build --release

FROM debian:trixie-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/rusty-noname .
COPY --from=builder /app/static ./static

CMD ["./rusty-noname"]
