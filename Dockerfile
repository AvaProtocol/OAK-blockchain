FROM ubuntu:20.04
WORKDIR /app
ADD ./target/release/oak-testnet .

ENTRYPOINT ["./oak-testnet", "--chain", "oak-testnet", "-d", "data/node01"]
CMD ["--name", "node01"]

