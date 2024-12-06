FROM rust:1.81.0-slim-bookworm

RUN mkdir -p /usr/src/app/jdtls_workspace && \
    chown -R 1000:1000 /usr/src/app/jdtls_workspace
# Set the working directory in the container
WORKDIR /usr/src/app

# Install runtime and development dependencies, Python, Node.js, and npm
RUN apt-get update && apt-get install \
    -y --no-install-recommends \
    pkg-config \
    libssl-dev \
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
    && apt-get install -y nodejs \
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
    && rm -rf /var/lib/apt/lists/* \
    && \
    curl -L -o /tmp/jdt-language-server.tar.gz https://www.eclipse.org/downloads/download.php?file=/jdtls/snapshots/jdt-language-server-latest.tar.gz && \
    mkdir -p /opt/jdtls && \
    tar -xzf /tmp/jdt-language-server.tar.gz -C /opt/jdtls --no-same-owner && \
    rm /tmp/jdt-language-server.tar.gz

# Add jdtls to PATH
ENV PATH="/opt/jdtls/bin:${PATH}"

# Install rust-analyzer and rustfmt
RUN rustup component add rust-analyzer
RUN rustup component add rustfmt

# Install jedi python language server, ast grep for tree sitter
RUN pip install jedi-language-server ast-grep-cli

# Install global npm packages
RUN npm install -g typescript-language-server typescript

# Install coverage tools
RUN cargo install cargo-llvm-cov
RUN rustup component add llvm-tools-preview

COPY Cargo.toml .

RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN touch src/lib.rs
RUN cargo build --release

RUN cargo install ast-grep --locked
COPY ./src/ast_grep /usr/src/ast_grep

# Document that the container listens on port 4444
EXPOSE 4444

# Set environment variables for logging and backtrace
ENV RUST_LOG=debug,glob=info,ignore=info
ENV RUST_BACKTRACE=1
ENV RA_LOG="/tmp/rust-analyzer.log"

CMD /bin/bash -c "touch Cargo.toml && cargo build --release"
