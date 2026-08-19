FROM rust:1.97.1 AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
Copy frontend ./frontend
RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /app

COPY --from=builder /app/target/release/rusty-noname .
COPY --from=builder /app/frontend ./frontend

CMD ["./rusty-noname"]
