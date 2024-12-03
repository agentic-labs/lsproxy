#!/bin/bash
# Kill existing containers
export PATH="/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:/Applications/Docker.app/Contents/Resources/bin:$PATH"

# Ensure Docker Desktop is running
if ! pgrep -f Docker.app > /dev/null; then
    echo "Docker Desktop is not running"
    open -a Docker
    # Wait for Docker to start
    for i in {1..30}; do
        if /usr/local/bin/docker info > /dev/null 2>&1; then
            break
        fi
        sleep 1
    done
fi

docker kill $(docker ps -q) 2>/dev/null || true

# Define port range
START_PORT=7000
END_PORT=7009

# Launch instances
for i in $(seq $START_PORT $END_PORT); do
    echo "Starting instance on port $i..."
    cd /Users/ivanovm/proj/lsproxy && nohup ./scripts/run.sh ~/proj/transformers "$i" & 
done

wait