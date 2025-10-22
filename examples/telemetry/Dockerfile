FROM rust:1.90-slim-bullseye

RUN apt update && apt install -y libssl-dev

WORKDIR /usr/src/app
ADD . .
