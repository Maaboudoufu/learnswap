# build
FROM rust:1.99 AS builder
WORKDIR /app
COPY ./ /app
RUN cargo build --release

# runtime

FROM debian:trixie-slim
COPY --from=builder /app/target/release/learnswap /usr/local/bin/learnswap
EXPOSE 3000
CMD ["/usr/local/bin/learnswap"]
