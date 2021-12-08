FROM ubuntu:20.04

ARG PROFILE=release
ARG BINARY=neumann-collator
ARG USERNAME=oak

LABEL maintainer "contact@oak.tech"
LABEL description="Binary for Neumann Collator"

# 1. Install curl to execute RPC call
# 2. Add user
# 3. Create data directory
# 4. Link data directory
RUN apt-get update && apt-get install -y curl && \
  useradd -m -u 1000 -U -s /bin/sh -d /$USERNAME $USERNAME && \
  mkdir -p /$USERNAME/.local/share && \
  mkdir /data && \
  chown -R $USERNAME:$USERNAME /data && \
  ln -s /data /$USERNAME/.local/share/$BINARY

USER $USERNAME

# Copy files
COPY --chown=$USERNAME ./target/$PROFILE/$BINARY /$USERNAME/$BINARY
COPY --chown=$USERNAME ./resources /$USERNAME/resources

# Open network port
# 30333 for parachain p2p
# 30334 for relaychain p2p
# 9933 for RPC call
# 9944 for Websocket
# 9615 for Prometheus (metrics)
EXPOSE 30333 30334 9933 9944 9615

# Specify volume
VOLUME ["/data"]

# Change work directory
WORKDIR $USERNAME

ENTRYPOINT ["./neumann-collator"]
