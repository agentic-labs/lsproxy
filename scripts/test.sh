#!/bin/bash

set -e  # Exit immediately if a command exits with a non-zero status

# Build the application using the build Dockerfile
docker build -t lsproxy-dev lsproxy

if ! docker run --rm -v "$(pwd)/lsproxy":/usr/src/app -v "$(pwd)":/mnt/lsproxy_root lsproxy-dev cargo test $@; then
    echo "Tests failed. Exiting."
    exit 1
fi
