# syntax=docker/dockerfile:1.7

FROM rust:1-bookworm AS build

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src; \
    printf 'fn main() {}\n' > src/main.rs; \
    cargo build --release; \
    rm -rf src
COPY src ./src
RUN touch src/main.rs
RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends ca-certificates chromium curl fonts-dejavu-core fonts-noto-cjk; \
    rm -rf /var/lib/apt/lists/*; \
    useradd --system --uid 10001 --home /nonexistent --shell /usr/sbin/nologin embeds

COPY --from=build /app/target/release/umamoe-embeds /usr/local/bin/umamoe-embeds

ENV UMAMOE_EMBEDS_BIND=0.0.0.0:8080
EXPOSE 8080

USER 10001

HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
    CMD curl -fsS "http://127.0.0.1:8080/healthz" >/dev/null || exit 1

ENTRYPOINT ["/usr/local/bin/umamoe-embeds"]
