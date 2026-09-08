FROM rust:1.97.1-trixie AS builder

WORKDIR /app

ENV SQLX_OFFLINE=true

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY static ./static
COPY templates ./templates
COPY migrations ./migrations
COPY .sqlx ./.sqlx

# Run tests (Unit Tests)
RUN cargo test --release

RUN cargo build --release

FROM debian:trixie-slim

WORKDIR /app
# Askama templates are already compiled into the binary.
COPY --from=builder /app/target/release/rusty-noname .

# Static files are needed at runtime.
COPY --from=builder /app/static ./static

CMD ["./rusty-noname"]
