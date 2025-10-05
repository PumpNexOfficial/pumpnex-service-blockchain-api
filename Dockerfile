FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY target/release/blockchain_api_v2 /app/blockchain_api_v2
COPY config/ /app/config/

EXPOSE 80
CMD ["./blockchain_api_v2"]
