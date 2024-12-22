FROM rust:1.82 as builder

WORKDIR /usr/src/app
COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libpq5 ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/payment-system /usr/local/bin/app
COPY --from=builder /usr/src/app/migrations /usr/local/bin/migrations

WORKDIR /usr/local/bin
CMD ["app"]