FROM rust:1.57-slim-buster AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev

WORKDIR /usr/src/casino
COPY Cargo.lock Cargo.toml /usr/src/casino/
COPY src /usr/src/casino/src

RUN cargo build --bin aggregator

FROM debian:buster-slim

RUN apt-get update && apt-get install -y pkg-config libssl-dev

EXPOSE 7000

COPY --from=builder /usr/src/casino/target/debug/aggregator /usr/local/bin/aggregator
CMD ["/usr/local/bin/aggregator"]
