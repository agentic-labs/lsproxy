FROM rust:1.81.0-slim-bookworm AS builder

WORKDIR /usr/src/app

RUN apt-get update && apt-get install -y curl

USER 1000

COPY --chown=1000:1000 . .

RUN cargo build --release

FROM rust:1.81.0-slim-bookworm

RUN mkdir -p /usr/src/app/jdtls_workspace && \
    chown -R 1000:1000 /usr/src/app/jdtls_workspace

# Install runtime dependencies, Python, Node.js, and npm
RUN apt-get update && apt-get install \
    -y --no-install-recommends \
    pkg-config \
    libssl3 \
    ca-certificates \
    git \
    python3 \
    python3-pip \
    python3-venv \
    curl \
    clangd \
    build-essential \
    gcc \
    g++ \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Config Python
RUN ln -sf /usr/bin/python3 /usr/bin/python
RUN rm /usr/lib/python3.11/EXTERNALLY-MANAGED

# Install Java
RUN apt-get update && \
    apt-get install \
    -y --no-install-recommends \
    openjdk-17-jdk \
    gradle \
    maven \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

RUN mkdir -p /opt/jdtls /mnt/workspace

# Install rust-analyzer
RUN rustup component add rust-analyzer
RUN rustup component add rustfmt

# Install jedi python language server, ast grep for tree sitter
RUN pip install jedi-language-server ast-grep-cli

# Install global npm packages
RUN npm install -g typescript-language-server typescript

RUN chown -R 1000:1000 /mnt/workspace /usr/local/cargo /opt/jdtls

# Add jdtls to PATH
ENV PATH="/opt/jdtls/bin:${PATH}"

USER 1000

RUN curl -L -o /tmp/jdt-language-server.tar.gz https://www.eclipse.org/downloads/download.php?file=/jdtls/snapshots/jdt-language-server-latest.tar.gz && \
    tar -xzf /tmp/jdt-language-server.tar.gz -C /opt/jdtls --no-same-owner && \
    rm /tmp/jdt-language-server.tar.gz

# Copy the binary
COPY --from=builder --chown=1000:1000 /usr/src/app/target/release/lsproxy .

# Copy ast-grep rules
COPY --chown=1000:1000 ./src/ast_grep  /usr/src/ast_grep

EXPOSE 4444
ENV RUST_LOG=info
ENV RA_LOG="/tmp/rust-analyzer.log"
CMD ["./lsproxy"]
