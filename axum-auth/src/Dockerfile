FROM rust:1.71-slim as build

WORKDIR /app

COPY . .

RUN cargo build --release

FROM debian:buster-slim

WORKDIR /usr/local/bin

COPY --from=build /app/target/release/axum-auth .

CMD ["./axum-auth"]
