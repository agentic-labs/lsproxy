#!/bin/bash
# Function to show usage
usage() {
    echo "Usage: $0 [-c command] [-o output.csv] [-r rate] [pid]"
    echo "Monitor system resources for a process and its children"
    echo
    echo "Options:"
    echo "  -c COMMAND    Run and monitor this command"
    echo "  -o FILE      Write output to CSV file"
    echo "  -r RATE      Sample rate in seconds (can be decimal, e.g. 0.1)"
    echo "  pid          Monitor an existing process ID"
    exit 1
}

# Parse arguments
COMMAND=""
PID=""
OUTPUT_FILE=""
SAMPLE_RATE="2"
while getopts "c:o:r:h" opt; do
    case $opt in
        c) COMMAND="$OPTARG" ;;
        o) OUTPUT_FILE="$OPTARG" ;;
        r) SAMPLE_RATE="$OPTARG" ;;
        h) usage ;;
        *) usage ;;
    esac
done
shift $((OPTIND-1))

# Get PID from argument if not using -c
if [ -z "$COMMAND" ] && [ $# -eq 1 ]; then
    PID=$1
fi

# Initialize CSV file if specified
if [ -n "$OUTPUT_FILE" ]; then
    echo "timestamp,pid,cores,memory_mb,command" > "$OUTPUT_FILE"
fi

# Create named pipe for command output
PIPE=$(mktemp -u)
mkfifo "$PIPE"

# Start the command if specified
if [ -n "$COMMAND" ]; then
    echo "Starting command: $COMMAND"
    # Run command and redirect its output through the pipe
    eval "$COMMAND > $PIPE 2>&1 &"
    PID=$!
    echo "Started with PID: $PID"
fi

# Validate PID
if [ -z "$PID" ]; then
    echo "Error: No PID specified or command failed to start"
    rm -f "$PIPE"
    exit 1
fi

# Check if process exists
if ! kill -0 "$PID" 2>/dev/null; then
    echo "Error: Process $PID not found"
    rm -f "$PIPE"
    exit 1
fi

echo "Monitoring process $PID..."
echo "Sample rate: ${SAMPLE_RATE} seconds"
if [ -n "$OUTPUT_FILE" ]; then
    echo "Writing to: $OUTPUT_FILE"
fi
echo "Press Ctrl+C to stop"
echo

# Cleanup function
cleanup() {
    echo -e "\nStopping monitor..."
    if [ -n "$COMMAND" ]; then
        echo "Killing process $PID and children..."
        pkill -P "$PID" 2>/dev/null
        kill "$PID" 2>/dev/null
    fi
    rm -f "$PIPE"
    exit 0
}
trap cleanup INT

# Function to calculate total RSS
get_total_rss() {
    pid=$1
    rss=0
    for p in $pid $(ps --ppid "$pid" -o pid= 2>/dev/null); do
        proc_rss=$(ps -p "$p" -o rss= 2>/dev/null | tr -d ' ')
        if [ -n "$proc_rss" ]; then
            rss=$((rss + proc_rss))
        fi
    done
    echo "$rss"
}

# Function to format process info
format_process() {
    pid=$1
    timestamp=$2
    display=$3
    stats=$(ps -p "$pid" -o pid=,pcpu=,rss=,comm= 2>/dev/null | sed 's/^[[:space:]]*//')
    if [ -n "$stats" ]; then
        read pid_out cpu rss cmd <<< "$stats"
        
        # Convert CPU percentage to cores
        cpu_cores=$(echo "$cpu" | awk '{printf "%.2f", $1/100}')
        # Convert RSS to MB
        mem_mb=$(echo "$rss" | awk '{printf "%.1f", $1/1024}')
        
        # Output to terminal if display is true
        if [ "$display" = "true" ]; then
            printf "%-6s %6s %8s  %s\n" "$pid_out" "$cpu_cores" "$mem_mb" "$cmd"
        fi
        
        # Output to CSV if enabled
        if [ -n "$OUTPUT_FILE" ]; then
            echo "${timestamp},${pid_out},${cpu_cores},${mem_mb},\"${cmd}\"" >> "$OUTPUT_FILE"
        fi
        
        echo "$cpu_cores" >> /tmp/cpu_cores_$$
    fi
}

# Start background process to read and display command output
if [ -n "$COMMAND" ]; then
    cat "$PIPE" &
fi

last_display_time=0
# Main monitoring loop
while true; do
    timestamp=$(date '+%Y-%m-%d %H:%M:%S.%N' | cut -b1-23)
    current_time=$(date +%s)
    
    # Determine if we should update the display
    display="false"
    if (( current_time - last_display_time >= 2 )); then
        display="true"
        last_display_time=$current_time
        echo "=== $timestamp ==="
        printf "%-6s %6s %8s  %-s\n" "PID" "CORES" "MEM(MB)" "COMMAND"
        echo "--------------------------------"
    fi
    
    # Create temporary file for CPU cores
    rm -f /tmp/cpu_cores_$$
    touch /tmp/cpu_cores_$$
    
    # Process monitoring
    format_process "$PID" "$timestamp" "$display"
    for child in $(ps --ppid "$PID" -o pid= 2>/dev/null); do
        format_process "$child" "$timestamp" "$display"
    done
    
    # Calculate and display totals
    total_rss=$(get_total_rss "$PID")
    total_mb=$(echo "$total_rss" | awk '{printf "%.1f", $1/1024}')
    total_cores=$(awk '{sum += $1} END {printf "%.2f", sum}' /tmp/cpu_cores_$$)
    process_count=$(( 1 + $(ps --ppid "$PID" -o pid= 2>/dev/null | wc -l) ))
    
    if [ "$display" = "true" ]; then
        echo "--------------------------------"
        echo "Total Processes: $process_count"
        echo "Total CPU Cores: ${total_cores}"
        echo "Total Memory: ${total_mb} MB"
        echo
    fi
    
    # Write totals to CSV if enabled
    if [ -n "$OUTPUT_FILE" ]; then
        echo "${timestamp},TOTAL,${total_cores},${total_mb},\"${process_count} processes\"" >> "$OUTPUT_FILE"
    fi
    
    rm -f /tmp/cpu_cores_$$
    sleep "$SAMPLE_RATE"
done
