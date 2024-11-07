#!/bin/bash

set -e  # Exit immediately if a command exits with a non-zero status

# Host port (external) defaults to 4444 if not specified
HOST_PORT=${2:-4444}

# Run the build
./scripts/build.sh

# Run the application with configurable host port, fixed container port
docker run --rm -p ${HOST_PORT}:4444 -v $1:/mnt/workspace -v "$(pwd)/lsproxy/target/release":/usr/src/app lsproxy-dev ./lsproxy
