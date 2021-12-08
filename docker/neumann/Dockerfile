FROM ubuntu:20.04

WORKDIR /app

# Copy files
ADD ./target/release/neumann-collator .
ADD ./resources resources

# Map data directory
RUN mkdir -p /data /root/.local/share && \
  ln -s /data /root/.local/share/neumann-collator

VOLUME ["/data"]

# Open network port
# 30333 for parachain p2p
# 30334 for relaychain p2p
# 9933 for RPC call
# 9944 for Websocket
# 9615 for Prometheus (metrics)
EXPOSE 30333 30334 9933 9944 9615

ENTRYPOINT ["./neumann-collator"]
