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

RUN cargo build --release

FROM debian:trixie-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/rusty-noname .
COPY --from=builder /app/static ./static

USER 10001:10001

CMD ["./rusty-noname"]
