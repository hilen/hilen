# syntax=docker/dockerfile:1.7

# Builds the demo wasm dist with trunk and serves it as static files.
# The context is the repo root, demo depends on the engine by path.
FROM rust:bookworm AS web-builder

# trunk comes as a ready program from its release page, checked by its
# hash. Never `cargo install trunk` in a Dockerfile: that compiles it from
# source, about 5 minutes, and again each time a line above it changes. This
# step is the first one, so nothing above it can change.
ARG TRUNK_VERSION=0.21.14
ARG TRUNK_SHA256=a67f4054b249fe9acc5fabc25de1aebf19783aca3ad6ff64bf34d7da44d0ea20
RUN curl -fsSL "https://github.com/trunk-rs/trunk/releases/download/v${TRUNK_VERSION}/trunk-x86_64-unknown-linux-musl.tar.gz" -o /tmp/trunk.tar.gz \
 && echo "${TRUNK_SHA256}  /tmp/trunk.tar.gz" | sha256sum -c - \
 && tar -xzf /tmp/trunk.tar.gz -C /usr/local/bin trunk \
 && rm /tmp/trunk.tar.gz
ENV CARGO_HOME=/usr/local/cargo
ENV CARGO_TARGET_DIR=/build/target
RUN apt-get update \
 && apt-get install -y --no-install-recommends pkg-config libssl-dev \
 && rm -rf /var/lib/apt/lists/*
# cargo clones the git dependencies through the git CLI so the anonymous
# https rewrite applies, a build container has no ssh key.
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true
RUN git config --global url."https://github.com/".insteadOf "git@github.com:"
WORKDIR /build
# The toolchain pin lands first so installing that nightly and its wasm
# target stays a cached layer, separate from the sources.
COPY rust-toolchain.toml ./rust-toolchain.toml
RUN rustup show && rustup target add wasm32-unknown-unknown
COPY . .
# webgl is a fallback, browsers with WebGPU still take WebGPU. trunk
# writes dist/ into the crate, outside the cached target dir, so the
# output survives the mount.
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked,id=hilen-demo-cargo-registry \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked,id=hilen-demo-cargo-git \
    --mount=type=cache,target=/build/target,sharing=locked,id=hilen-demo-cargo-target \
    cd demo && trunk build --release --features webgl --public-url /

FROM nginx:1.27-alpine AS runtime
COPY demo/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=web-builder /build/demo/dist /usr/share/nginx/html
EXPOSE 80
