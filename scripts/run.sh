#!/bin/bash
set -e  # Exit immediately if a command exits with a non-zero status

# Color definitions
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Help function
print_usage() {
    echo -e "${BLUE}Usage: $0 [--no-auth] <workspace_path>${NC}"
    echo -e "  --no-auth    : Disable authentication (sets USE_AUTH=false)"
    echo -e "  workspace_path: Path to the workspace directory"
}

# Parse command line arguments
WORKSPACE_PATH=""
USE_AUTH=true

while [[ $# -gt 0 ]]; do
    case $1 in
        --no-auth)
            USE_AUTH=false
            shift
            ;;
        -h|--help)
            print_usage
            exit 0
            ;;
        *)
            if [ -z "$WORKSPACE_PATH" ]; then
                WORKSPACE_PATH="$1"
            else
                echo -e "${YELLOW}Warning: Unexpected argument: $1${NC}"
            fi
            shift
            ;;
    esac
done

# Check if workspace path is provided
if [ -z "$WORKSPACE_PATH" ]; then
    echo -e "${YELLOW}Error: Workspace path is required${NC}"
    print_usage
    exit 1
fi

if [ "$USE_AUTH" = true ]; then
    # Generate JWT token for Swagger login
    echo -e "${BLUE}Generating JWT token for Swagger UI login...${NC}"
    JWT_SECRET=test_secret ./scripts/generate_jwt.py
    echo -e "${YELLOW}Note: To disable authentication, you can use the --no-auth flag${NC}"

    # Ask for confirmation to continue - fixed syntax
    echo -e "${GREEN}Token has been generated. Press Enter to continue with application startup${NC}"
    read -p "(Ctrl+C to cancel)..."

    AUTH_ENV="-e JWT_SECRET=test_secret"
else
    echo -e "${BLUE}Running in no-auth mode...${NC}"
    AUTH_ENV="-e USE_AUTH=false"
fi

echo -e "${BLUE}Starting application...${NC}"

# Run the build
./scripts/build.sh

# Run the application
docker run --rm -p 4444:4444 \
    -v "$WORKSPACE_PATH:/mnt/workspace" \
    $AUTH_ENV \
    -v "$(pwd)/lsproxy/target/release":/usr/src/app \
    lsproxy-dev ./lsproxy
