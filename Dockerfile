# syntax=docker/dockerfile:1.2
FROM rust:1.58-slim
ENV PATH="/usr/local/bin:${PATH}"

RUN apt update && apt install -y pkg-config libssl-dev

WORKDIR /usr/src/app
COPY . .

WORKDIR /usr/src/app/examples/metric-aggregator
RUN --mount=type=cache,target=/usr/src/app/target \
    --mount=type=cache,target=/usr/src/app/examples/metric-aggregator/target/ \
    cargo build --release && \
    install target/release/client target/release/server target/release/load_client /usr/local/bin
