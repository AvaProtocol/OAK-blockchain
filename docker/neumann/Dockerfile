FROM ubuntu:20.04

ARG PROFILE=release
ARG BINARY=neumann-collator
ARG USERNAME=oak

LABEL maintainer "contact@oak.tech"
LABEL description="Binary for Neumann Collator"

# Add user and create data directory
RUN useradd -m -u 1000 -U -s /bin/sh -d /$USERNAME $USERNAME && \
	mkdir -p /$USERNAME/.local/share && \
	mkdir /data && \
	chown -R $USERNAME:$USERNAME /data && \
	ln -s /data /$USERNAME/.local/share/$BINARY && \
  rm -rf /usr/bin /usr/sbin

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

ENTRYPOINT ["./neumann-collator"]
