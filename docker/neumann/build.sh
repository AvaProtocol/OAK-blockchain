#!/usr/bin/env bash
# Please execute this script in the project root directory.
set -e

# The following line ensures we run from the project root
PROJECT_ROOT=`git rev-parse --show-toplevel`
cd $PROJECT_ROOT

# Find the current version from Cargo.toml
VERSION=`grep "^version" ./node/Cargo.toml | egrep -o "([0-9\.]+)"`
USER=oaknetwork
PROJECT=neumann

# Build the image
echo "Building ${USER}/${PROJECT}:latest docker image, hang on!"
time docker build -f ./docker/neumann/Dockerfile -t ${USER}/${PROJECT}:latest .
docker tag ${USER}/${PROJECT}:latest ${USER}/${PROJECT}:${VERSION}

# Show the list of available images for this repo
echo "Image is ready"
docker images | grep ${PROJECT}
