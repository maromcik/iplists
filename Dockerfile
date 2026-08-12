# ---------- Rust base ----------
# Shared base: toolchain, system deps and cargo-chef (installed once, cached
# for all stages derived from here).
FROM rust:1.96-slim AS base

RUN apt-get update && apt-get install -y \
    cmake \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-chef --version 0.1.73

WORKDIR /usr/src/app


# ---------- Rust: plan ----------
# Compute the dependency recipe from the manifests. This layer only
# invalidates when Cargo.toml/Cargo.lock change.
FROM base AS planner

COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./src ./src

RUN cargo chef prepare --recipe-path recipe.json


# ---------- Rust: build ----------
FROM base AS builder

# Cook the dependencies first: cached as long as the recipe is unchanged.
COPY --from=planner /usr/src/app/recipe.json recipe.json

RUN cargo chef cook --release --recipe-path recipe.json

# Only now copy the actual sources; rebuilding them reuses the cooked deps.
COPY ./src ./src
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
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

COPY --from=builder /usr/src/app/target/release/iplists /usr/local/bin

COPY ./static ./static
COPY --from=frontend-builder /usr/src/app/frontend/dist ./frontend/dist
COPY --from=frontend-builder /usr/src/app/static/css/output.css ./static/css/output.css

RUN mkdir -p /opt/iplists

VOLUME ["/opt/iplists"]

CMD ["/usr/local/bin/iplists", "-c", "/opt/iplists/iplists.yaml"]
