FROM rust:1.81-alpine AS builder
ARG RUSTFLAGS="-C link-arg=-fuse-ld=mold"
RUN apk add libressl-dev pkgconfig musl musl-dev
RUN rustup target add x86_64-unknown-linux-musl
WORKDIR /usr/app/geoip
COPY Cargo.lock Cargo.toml /usr/app/geoip/
RUN --mount=type=cache,target=/usr/local/cargo/registry \
        --mount=type=cache,target=/usr/app/target \
        RUSTFLAGS="--cfg tracing_unstable" cargo fetch --manifest-path /usr/app/geoip/Cargo.toml
COPY ./src /usr/app/geoip/src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/app/target \
    RUSTFLAGS="--cfg tracing_unstable" cargo build --target=x86_64-unknown-linux-musl --release && cp /usr/app/geoip/target/x86_64-unknown-linux-musl/release/geoip-server-rs /geoip-server-rs
FROM alpine:3.20.2 AS release
COPY --from=builder /geoip-server-rs .
CMD ["sh", "-c", "./geoip-server-rs"]
