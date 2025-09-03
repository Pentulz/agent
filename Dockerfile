ARG RUST_VERSION="1.89"
ARG DEBIAN_VERSION="trixie"
ARG FFUF_VERSION="2.1.0"

FROM rust:${RUST_VERSION} AS builder
ARG FFUF_VERSION
WORKDIR /usr/src/agent
COPY . .

RUN cd /tmp/ \
  && wget https://github.com/ffuf/ffuf/releases/download/v${FFUF_VERSION}/ffuf_${FFUF_VERSION}_linux_amd64.tar.gz \
  && tar -xzf ffuf_${FFUF_VERSION}_linux_amd64.tar.gz \
  && mv ffuf /usr/bin/ \
  && rm -rf ffuf_${FFUF_VERSION}_linux_amd64.tar.gz ffuf

RUN cargo install --path .

FROM debian:${DEBIAN_VERSION}-slim
RUN apt-get update && apt-get install -y nmap tshark && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/cargo/bin/agent /usr/local/bin/agent
COPY --from=builder /usr/bin/ffuf /usr/local/bin/ffuf

CMD ["tail", "-f", "/dev/null"]

