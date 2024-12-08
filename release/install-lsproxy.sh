#!/bin/bash
set -e

LSPROXY_VERSION="0.1.10"

# Function to detect architecture
detect_arch() {
    local arch=$(uname -m)
    case $arch in
        x86_64)
            echo "amd64"
            ;;
        aarch64|arm64)
            echo "arm64"
            ;;
        *)
            echo "Unsupported architecture: $arch"
            exit 1
            ;;
    esac
}

# Check if running as root
check_root() {
    if [ "$(id -u)" -ne 0 ]; then
        echo "Error: This script must be run as root"
        exit 1
    fi
}

# Function to install system dependencies
install_system_deps() {
    echo "Installing system dependencies..."
    apt-get update
    DEBIAN_FRONTEND=noninteractive apt-get install -y \
        pkg-config \
        libssl3 \
        ca-certificates \
        git \
        curl \
        clangd \
        build-essential \
        gcc \
        g++
}

# Function to install python
install_python() {
   echo "Installing python and packages..."
   DEBIAN_FRONTEND=noninteractive apt-get install -y \
       python3 \
       python3-pip

   ln -sf /usr/bin/python3 /usr/bin/python

   PY_VERSION=$(python --version | cut -d' ' -f2 | cut -d'.' -f1,2)
   MANAGED_FILE="/usr/lib/python${PY_VERSION}/EXTERNALLY-MANAGED"

   if [ -f "$MANAGED_FILE" ]; then
       rm "$MANAGED_FILE"
   else
       echo "Warning: EXTERNALLY-MANAGED file not found at $MANAGED_FILE"
   fi

   pip3 install jedi-language-server ast-grep-cli
}

# Function to install Node.js
install_nodejs() {
    echo "Installing Node.js..."
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash -
    apt-get install -y nodejs
}

# Function to install Java and JDT
install_java() {
    echo "Installing Java and JDT..."
    DEBIAN_FRONTEND=noninteractive apt-get install -y openjdk-17-jdk gradle maven
    curl -L -o /tmp/jdt-language-server.tar.gz \
        "https://www.eclipse.org/downloads/download.php?file=/jdtls/snapshots/jdt-language-server-latest.tar.gz"
    mkdir -p /opt/jdtls
    tar -xzf /tmp/jdt-language-server.tar.gz -C /opt/jdtls --no-same-owner
    rm /tmp/jdt-language-server.tar.gz
    
    # Add jdtls to PATH
    echo 'export PATH="/opt/jdtls/bin:${PATH}"' >> /etc/profile.d/jdtls.sh
}

# Function to install Node.js dependencies
install_node_deps() {
    echo "Installing Node.js dependencies..."
    npm install -g typescript-language-server typescript
}

# Function to install Rust tooling
install_rust_tools() {
    echo "Installing Rust analysis tools..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    . $HOME/.cargo/env
    rustup component add rust-analyzer
    rustup component add rustfmt
}

# Function to download and install LSProxy binary
install_lsproxy() {
    local arch=$(detect_arch)
    local binary_url="https://github.com/agentic-labs/lsproxy/releases/download/${LSPROXY_VERSION}/lsproxy-${LSPROXY_VERSION}-linux-${arch}"
    
    echo "Downloading LSProxy binary for Linux ${arch}..."
    curl -L -o /usr/local/bin/lsproxy "${binary_url}"
    chmod +x /usr/local/bin/lsproxy
}

# Function to install ast_grep configuration
install_ast_grep_config() {
    local config_url="https://github.com/agentic-labs/lsproxy/releases/download/${LSPROXY_VERSION}/lsproxy-${LSPROXY_VERSION}-ast-grep-rules.tar.gz"
    local dest_dir="/usr/src/ast_grep"

    echo "Downloading ast_grep configuration and rules..."
    mkdir -p "$dest_dir"
    curl -L -o /tmp/ast_grep.tar.gz "$config_url"

    echo "Extracting ast_grep configuration and rules..."
    tar -xzf /tmp/ast_grep.tar.gz -C "$dest_dir" --no-same-owner
    rm /tmp/ast_grep.tar.gz
}

# Function to clean up
cleanup() {
    echo "Cleaning up..."
    apt-get clean
    rm -rf /var/lib/apt/lists/*
}

# Main installation
main() {
    echo "Installing LSProxy version ${LSPROXY_VERSION} for Linux..."
    check_root
    
    install_system_deps
    install_python
    install_nodejs
    install_java
    install_node_deps
    install_rust_tools
    install_lsproxy
    install_ast_grep_config
    cleanup
    
    echo "LSProxy installation complete!"
}

main
