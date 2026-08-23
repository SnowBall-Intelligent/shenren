# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS backend
WORKDIR /src/backend
COPY backend/ ./
RUN cargo build --release --locked \
    && cp target/release/shenren /tmp/shenren

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home-dir /app --create-home shenren \
    && mkdir -p /data/uploads /app/data/logs/admin \
    && chown -R shenren:shenren /app /data

WORKDIR /app
COPY --from=backend /tmp/shenren /app/shenren

ENV BIND_ADDR=0.0.0.0:3000 \
    UPLOADS_DIR=/data/uploads \
    COOKIE_SECURE=false \
    DATABASE_URL=mysql://shenren:shenren@host.docker.internal:3306/shenren

EXPOSE 3000
VOLUME ["/data/uploads", "/app/data/logs"]
USER shenren
HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD curl -fsS "http://127.0.0.1:3000/api/site" || exit 1
ENTRYPOINT ["/app/shenren"]
