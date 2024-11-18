#!/bin/bash

set -e  # Exit immediately if a command exits with a non-zero status

# Default port if not specified
PORT=${2:-4444}

# Run the build
./scripts/build.sh

# Run the application
docker run --rm -p ${PORT}:4444 -v $1:/mnt/workspace -v "$(pwd)/lsproxy/target/release":/usr/src/app lsproxy-dev ./lsproxy
