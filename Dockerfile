FROM rust:bullseye as builder
RUN git clone http://192.168.1.2:8081/c0d3-m4513r/rust-dc-bot.git
WORKDIR rust-dc-bot
RUN cargo install --path .

FROM debian:bullseye-slim
COPY --from=builder /rust-dc-bot/target/release/untitled /untitled
WORKDIR "/data"
CMD ["/untitled"]