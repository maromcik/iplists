# ---------- Rust ----------
FROM rust:1.96-slim AS builder

RUN apt-get update && apt-get install -y \
    cmake \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

COPY ./src ./src
COPY ./templates ./templates
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock

RUN cargo build --release


# ---------- Frontend ----------
FROM node:22-bookworm-slim AS frontend-builder

WORKDIR /usr/src/app

COPY ./frontend/package*.json ./frontend/
RUN cd frontend && npm ci

COPY ./frontend ./frontend
COPY ./static ./static

# Build Svelte
RUN cd frontend && npm run build

# Build Tailwind CSS
RUN cd frontend && npx tailwindcss \
    -i style.css \
    -o ../static/css/output.css \
    --minify


# ---------- Runtime ----------
FROM debian:trixie-slim AS runtime

RUN apt-get update && apt-get install -y \
    zip \
    pkg-config \
    libssl-dev \
    clang \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

COPY --from=builder /usr/src/app/target/release/iplists /usr/local/bin

COPY ./static ./static
COPY --from=frontend-builder /usr/src/app/frontend/dist ./frontend/dist
COPY --from=frontend-builder /usr/src/app/static/css/output.css ./static/css/output.css

RUN mkdir -p /opt/iplists

COPY iplists.yaml /opt/iplists/iplists.yaml

VOLUME ["/opt/iplists"]

CMD ["/usr/local/bin/iplists", "-c", "/opt/iplists/iplists.yaml"]
