# This is the build stage for OAK. Here we create the binary in a temporary image.
FROM docker.io/paritytech/ci-linux:production as builder

WORKDIR /build
COPY . /build

RUN cargo install --git https://github.com/chevdor/srtool-cli
RUN srtool build -p neumann-runtime

