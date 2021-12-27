FROM denbeigh2000/casino:base AS base

COPY Cargo.lock Cargo.toml /usr/src/casino/
COPY src /usr/src/casino/src

ARG CARGO_ARGS=""

RUN cargo build --bin aggregator $CARGO_ARGS

FROM debian:buster-slim

RUN apt-get update && apt-get install -y pkg-config libssl-dev ca-certificates

EXPOSE 7000

COPY --from=base /usr/src/casino/target/debug/aggregator /usr/local/bin/aggregator
CMD ["/usr/local/bin/aggregator"]

