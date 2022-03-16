# This is the build stage for OAK. Here we create the binary in a temporary image.
FROM docker.io/paritytech/ci-linux:production as builder

WORKDIR /build
COPY . /build

RUN cargo build --locked --release --features turing-node
RUN /build/target/release/oak-collator build-spec --disable-default-bootnode --chain=turing-dev --raw > turing-dev-chain-spec.json
RUN /build/target/release/oak-collator build-spec --disable-default-bootnode --chain=turing-staging --raw > turing-staging-chain-spec.json
RUN /build/target/release/oak-collator build-spec --disable-default-bootnode --chain=turing-live --raw > turing-live-chain-spec.json

# This is the 2nd stage: a very small image where we copy the OAK binary."
FROM docker.io/library/ubuntu:20.04

ARG USERNAME=oak
ARG PROFILE=release
ARG BINARY=oak-collator

LABEL maintainer "contact@oak.tech"
LABEL description="Binary for Turing Collator"

# 1. Install curl to execute RPC call
# 2. Add user
# 3. Create data directory
# 4. Link data directory
# 5. Delete binaries except curl
RUN apt-get update && apt-get install -y curl && \
  useradd -m -u 1000 -U -s /bin/sh -d /$USERNAME $USERNAME && \
  mkdir -p /$USERNAME/.local/share && \
  mkdir /data && \
  chown -R $USERNAME:$USERNAME /data && \
  ln -s /data /$USERNAME/.local/share/$BINARY && \
  find /usr/bin/* /usr/sbin/* | grep -v curl | xargs rm

USER $USERNAME

# Copy files
COPY --chown=$USERNAME --from=builder /build/target/release/$BINARY  /$USERNAME/$BINARY
COPY --chown=$USERNAME ./resources /$USERNAME/resources
COPY --chown=$USERNAME --from=builder /build/turing-dev-chain-spec.json  /$USERNAME/resources/turing-dev-chain-spec.json
COPY --chown=$USERNAME --from=builder /build/turing-staging-chain-spec.json  /$USERNAME/resources/turing-staging-chain-spec.json
COPY --chown=$USERNAME --from=builder /build/turing-live-chain-spec.json /$USERNAME/resources/turing-live-chain-spec.json

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
WORKDIR /$USERNAME

ENTRYPOINT ["./oak-collator"]
