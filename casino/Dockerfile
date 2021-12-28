FROM registry.digitalocean.com/csgo/base AS base

RUN apt-get update && apt-get install -y pkg-config libssl-dev

WORKDIR /usr/src/casino
COPY Cargo.lock Cargo.toml /usr/src/casino/
COPY src /usr/src/casino/src

RUN cargo build --bin aggregator

# RUN cp target/debug/aggregator /usr/local/bin/aggregator

FROM debian:buster-slim

RUN apt-get update && apt-get install -y pkg-config libssl-dev

COPY --from=base /usr/src/casino/target/debug/aggregator /usr/local/bin/aggregator

EXPOSE 7000

CMD ["/usr/local/bin/aggregator"]

