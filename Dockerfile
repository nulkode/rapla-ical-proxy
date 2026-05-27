FROM rust:slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl unzip pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL https://bun.sh/install | bash
ENV PATH="/root/.bun/bin:$PATH"

WORKDIR /app
COPY . .

RUN cd web && bun install && bun run build
RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/rapla-ical-proxy /usr/local/bin/rapla-ical-proxy

ENV RAPLA_ADDRESS=0.0.0.0:8080
ENV RAPLA_DB_PATH=/data/rapla.db
RUN mkdir -p /data
VOLUME ["/data"]
EXPOSE 8080

ENTRYPOINT [ "rapla-ical-proxy" ]
