# syntax=docker/dockerfile:1.7

# Builds the demo wasm dist with trunk and serves it as static files.
# The context is the repo root, demo depends on the engine by path.
FROM rust:bookworm AS web-builder
ENV CARGO_HOME=/usr/local/cargo
ENV CARGO_TARGET_DIR=/build/target
RUN apt-get update \
 && apt-get install -y --no-install-recommends pkg-config libssl-dev \
 && rm -rf /var/lib/apt/lists/*
RUN cargo install --locked trunk
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
