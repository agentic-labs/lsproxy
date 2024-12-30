#!/bin/bash

set -e  # Exit immediately if a command exits with a non-zero status

# Build the application using the build Dockerfile
docker build -t lsproxy-dev lsproxy

# Create coverage directory
mkdir -p "$(pwd)/lsproxy/coverage"

# Run coverage analysis in Docker
docker run --rm \
    -v "$(pwd)/lsproxy":/usr/src/app \
    -v "$(pwd)":/mnt/lsproxy_root \
    --name coverage-container \
    lsproxy-dev sh -c "cd /usr/src/app && RUST_BACKTRACE=1 cargo llvm-cov --lib --html --output-dir=/usr/src/app/coverage"

# Check if coverage report was generated
if [ ! -f "$(pwd)/lsproxy/coverage/index.html" ]; then
    echo "Coverage report generation failed."
    exit 1
fi

# Copy coverage report from container
docker cp coverage-container:/usr/src/app/coverage/. "$(pwd)/lsproxy/coverage/"
