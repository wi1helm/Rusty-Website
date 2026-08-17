FROM rust:1.97.1

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build

CMD ["./target/debug/rusty-noname"]