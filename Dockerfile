FROM docker.io/library/rust:1.100-trixie@sha256:223cc32ddca8f3ee90c436dbc94b6b5f052ea7f9653c5201e9612f08ad7d21b8 AS build
WORKDIR /build
COPY . .
RUN cargo build --locked --release --package ccc-native

FROM docker.io/library/debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /build/target/release/ccc-native /usr/local/bin/ccc-native
USER 65532:65532
EXPOSE 8080
HEALTHCHECK --interval=20s --timeout=2s \
    CMD ["curl", "--fail", "--silent", "--show-error", "--max-time", "1", "--noproxy", "*", "http://localhost:8080/ping"]
ENTRYPOINT ["/usr/local/bin/ccc-native"]
