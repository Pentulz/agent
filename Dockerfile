ARG RUST_VERSION="1.89"
ARG DEBIAN_VERSION="trixie"

FROM rust:${RUST_VERSION} AS builder
WORKDIR /usr/src/agent
COPY . .
RUN cargo install --path .

FROM debian:${DEBIAN_VERSION}-slim
RUN apt-get update && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/agent /usr/local/bin/agent
CMD ["agent"]

