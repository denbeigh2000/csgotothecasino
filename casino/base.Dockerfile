FROM rust:1.57-slim-buster AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev

WORKDIR /usr/src/casino
COPY Cargo.lock Cargo.toml /usr/src/casino/
COPY src /usr/src/casino/src

RUN cargo build
