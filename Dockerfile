FROM docker.io/rustlang/rust:nightly-bullseye AS build
WORKDIR /source
RUN apt-get update && apt-get install -y protobuf-compiler
COPY . .
RUN cargo build --release

FROM debian:bullseye-20230411-slim
ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y ca-certificates --no-install-recommends && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build --chown=1001:1001 /source/target/release/mumble-telegram-bot /app/
USER 1001
ENTRYPOINT [ "/app/mumble-telegram-bot" ]
