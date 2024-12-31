import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
import argparse

def plot_performance_data(csv_file):
    # Read the CSV file
    df = pd.read_csv(csv_file)
    
    # Convert timestamp to datetime with ms precision
    df['timestamp'] = pd.to_datetime(df['timestamp'], format='%Y-%m-%d %H:%M:%S.%f')
    
    # Calculate the relative time in seconds from start with ms precision
    start_time = df['timestamp'].min()
    df['relative_time'] = (df['timestamp'] - start_time).dt.total_seconds()
    
    # Get unique processes (excluding rows where pid is 'TOTAL')
    processes = df[df['pid'] != 'TOTAL']['command'].unique()
    
    # Create figure with two subplots sharing x axis
    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(12, 8), sharex=True)
    fig.suptitle('Process Performance Metrics', fontsize=14)
    
    # Color map for consistent colors across plots
    colors = plt.cm.tab10(np.linspace(0, 1, len(processes) + 1))
    
    # Plot CPU cores usage
    for i, process in enumerate(processes):
        process_data = df[df['command'] == process]
        ax1.plot(process_data['relative_time'], process_data['cores'], 
                label=process, color=colors[i], linewidth=1)
    
    # Plot total CPU usage
    total_data = df[df['pid'] == 'TOTAL']
    ax1.plot(total_data['relative_time'], total_data['cores'],
             label='Total', color=colors[-1], linewidth=2, linestyle='--')
    
    ax1.set_ylabel('CPU Cores')
    ax1.grid(True, alpha=0.3)
    ax1.legend(bbox_to_anchor=(1.05, 1), loc='upper left')
    
    # Plot Memory usage
    for i, process in enumerate(processes):
        process_data = df[df['command'] == process]
        ax2.plot(process_data['relative_time'], process_data['memory_mb'],
                label=process, color=colors[i], linewidth=1)
    
    # Plot total memory usage
    ax2.plot(total_data['relative_time'], total_data['memory_mb'],
             label='Total', color=colors[-1], linewidth=2, linestyle='--')
    
    ax2.set_xlabel('Time (seconds)')
    ax2.set_ylabel('Memory (MB)')
    ax2.grid(True, alpha=0.3)
    ax2.legend(bbox_to_anchor=(1.05, 1), loc='upper left')
    
    # Calculate and display actual sample rate
    time_diffs = np.diff(df[df['pid'] == 'TOTAL']['relative_time'])
    actual_rate = np.mean(time_diffs)
    plt.figtext(0.02, 0.02, f'Avg sample rate: {actual_rate:.3f}s', 
                fontsize=8, ha='left')
    
    # Adjust layout to prevent label cutoff
    plt.tight_layout()
    
    return fig

def main():
    parser = argparse.ArgumentParser(description='Plot process performance metrics from CSV')
    parser.add_argument('csv_file', help='Path to the CSV file containing performance data')
    parser.add_argument('-o', '--output', help='Output file path (optional)')
    args = parser.parse_args()
    
    # Create the plot
    fig = plot_performance_data(args.csv_file)
    
    # Save or show the plot
    if args.output:
        fig.savefig(args.output, bbox_inches='tight', dpi=300)
        print(f"Plot saved to {args.output}")
    else:
        plt.show()

if __name__ == "__main__":
    main()
