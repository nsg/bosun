# syntax=docker/dockerfile:1
# Built per-arch by .github/workflows/build.yml and published to GHCR as the
# Home Assistant add-on image. The build stage runs under the target platform
# (via buildx/QEMU), so `cargo build` produces a native static musl binary.
FROM rust:1-alpine AS build
RUN apk add --no-cache musl-dev
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release && cp target/release/bosun /bosun

FROM alpine:3.20
RUN apk add --no-cache jq
COPY --from=build /bosun /usr/bin/bosun
COPY bosun.example.toml /usr/share/bosun/bosun.example.toml
COPY docker/run.sh /run.sh
RUN chmod +x /run.sh
ENTRYPOINT ["/run.sh"]
