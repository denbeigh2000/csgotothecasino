FROM rust:1.57-slim-buster

RUN apt-get update && apt-get install -y pkg-config libssl-dev

WORKDIR /usr/src/casino
COPY Cargo.lock Cargo.toml /usr/src/casino/
COPY src /usr/src/casino/src

ARG CARGO_ARGS=""

RUN cargo build --bin aggregator $CARGO_ARGS

RUN cp target/debug/aggregator /usr/local/bin/aggregator

EXPOSE 7000

CMD ["/usr/local/bin/aggregator"]

