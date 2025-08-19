FROM rust:1.89 AS builder
WORKDIR /usr/src/agent
COPY . .
RUN cargo install --path .

FROM debian:trixie-slim
RUN apt-get update && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/agent /usr/local/bin/agent
CMD ["agent"]

