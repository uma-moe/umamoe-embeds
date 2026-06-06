# syntax=docker/dockerfile:1.7

FROM rust:1-bookworm AS build

WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends ca-certificates; \
    rm -rf /var/lib/apt/lists/*; \
    useradd --system --uid 10001 --home /nonexistent --shell /usr/sbin/nologin embeds

COPY --from=build /app/target/release/umamoe-embeds /usr/local/bin/umamoe-embeds

ENV UMAMOE_EMBEDS_BIND=0.0.0.0:8080
EXPOSE 8080

USER 10001
ENTRYPOINT ["/usr/local/bin/umamoe-embeds"]
