FROM rust:1.81-alpine AS builder
ARG RUSTFLAGS="-C link-arg=-fuse-ld=mold"
ARG LICENSE
RUN apk add libressl-dev pkgconfig musl musl-dev gzip curl
RUN rustup target add x86_64-unknown-linux-musl
WORKDIR /usr/app/geoip
RUN if [ -n "$LICENSE" ]; then \
        wget -O GeoLite2-City.mmdb.tar.gz "https://download.maxmind.com/app/geoip_download?edition_id=GeoLite2-City&license_key=${LICENSE}&suffix=tar.gz" && echo "download complete" && ls -lash && pwd && \
        tar xvzf GeoLite2-City.mmdb.tar.gz && find . -name GeoLite2-City.mmdb | xargs -I {} cp {} ./ ;fi
COPY Cargo.lock Cargo.toml /usr/app/geoip/
RUN --mount=type=cache,target=/usr/local/cargo/registry \
        --mount=type=cache,target=/usr/app/geoip/target \
        RUSTFLAGS="--cfg tracing_unstable" cargo fetch --manifest-path /usr/app/geoip/Cargo.toml
COPY ./src /usr/app/geoip/src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/app/geoip/target \
    RUSTFLAGS="--cfg tracing_unstable" cargo build --target=x86_64-unknown-linux-musl --release && cp /usr/app/geoip/target/x86_64-unknown-linux-musl/release/geoip-server-rs /geoip-server-rs
FROM alpine:3.20.2 AS release
COPY --from=builder /geoip-server-rs .
COPY --from=builder /usr/app/geoip/GeoLite2-City.mmdb .
CMD ["sh", "-c", "./geoip-server-rs"]
