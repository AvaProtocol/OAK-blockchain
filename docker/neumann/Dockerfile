FROM ubuntu:20.04
WORKDIR /app
ADD ./target/release/neumann-collator .

ENTRYPOINT ["./neumann-collator", "-d", "data/node"]
