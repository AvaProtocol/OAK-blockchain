FROM ubuntu:20.04
WORKDIR /app
ADD ./target/release/oak .

ENTRYPOINT ["./oak", "-d", "data/node"]
