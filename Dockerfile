FROM rust:1.95-alpine AS builder

RUN apk add --no-cache \
    build-base \
    musl-dev

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --locked

FROM scratch

COPY --from=builder /app/target/release/zitadel-tui /usr/local/bin/zitadel-tui

ENTRYPOINT ["zitadel-tui"]
