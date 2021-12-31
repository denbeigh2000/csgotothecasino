FROM rust:1.57-slim-buster

RUN apt-get update && apt-get install -y pkg-config libssl-dev

WORKDIR /usr/src/casino
COPY Cargo.lock Cargo.toml /usr/src/casino/
COPY aggregator /usr/src/casino/aggregator
COPY bootstrap /usr/src/casino/bootstrap
COPY cache /usr/src/casino/cache
COPY collector /usr/src/casino/collector
COPY csgofloat /usr/src/casino/csgofloat
COPY logging /usr/src/casino/logging
COPY steam /usr/src/casino/steam
COPY store /usr/src/casino/store

RUN cargo build --bin aggregator --release
