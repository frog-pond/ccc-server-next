FROM docker.io/rust:alpine AS build

RUN apk add --no-cache musl-dev openssl-dev

WORKDIR /opt/ccc/

COPY Cargo.toml Cargo.lock ./
COPY e2e ./e2e/
COPY ccc-types ./ccc-types/
COPY ccc-parse ./ccc-parse/
COPY ccc-handlers ./ccc-handlers/
COPY ccc-routes ./ccc-routes/
COPY ccc-server ./ccc-server/

RUN cargo fetch

RUN cargo build --release


FROM alpine:latest

EXPOSE 3000

RUN apk add --no-cache openssl ca-certificates libc6-compat

COPY --from=build /opt/ccc/target/release/ccc-server /opt/ccc/ccc-server

ENTRYPOINT ['/opt/ccc/ccc-server']
