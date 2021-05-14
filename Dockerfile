FROM ubuntu:20.04
WORKDIR /app
ADD ./target/release/node-template .

ENTRYPOINT ["./node-template", "--chain", "oak-testnet", "-d", "data/node01", "--name", "node01"]

