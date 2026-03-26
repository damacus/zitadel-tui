FROM rust:1.89-alpine AS builder

RUN apk add --no-cache \
    build-base \
    musl-dev

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM alpine:3.23

RUN apk add --no-cache ca-certificates

WORKDIR /app

COPY --from=builder /app/target/release/zitadel-tui /usr/local/bin/zitadel-tui

ENTRYPOINT ["zitadel-tui"]
