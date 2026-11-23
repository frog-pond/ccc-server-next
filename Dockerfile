FROM docker.io/library/rust:1.100-trixie AS build
WORKDIR /build
COPY . .
RUN cargo build --locked --release --package ccc-native

FROM docker.io/library/debian:trixie-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /build/target/release/ccc-native /usr/local/bin/ccc-native
USER 65532:65532
EXPOSE 80
HEALTHCHECK --interval=20s --timeout=2s \
    CMD ["curl", "--fail", "--silent", "--show-error", "--max-time", "1", "--noproxy", "*", "http://localhost:80/ping"]
ENTRYPOINT ["/usr/local/bin/ccc-native"]
