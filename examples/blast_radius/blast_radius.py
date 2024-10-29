"""
Blast Radius is a tool for analyzing the impact of code changes between Git commits.

It works by:
1. Clones a Git repository and checks out two commits to compare
2. Analyzes the diff between commits to identify modified lines of code
3. Uses lsproxy to parse code symbols and track dependencies
4. Finds all symbol definitions (functions, classes, variables) that were changed
5. Traces references through the codebase to identify code impacted by the changes
6. Generates a "blast radius" report showing the full scope of impact
"""

import argparse
import shutil
import subprocess
import tempfile
from time import sleep
import logging
import requests

from hierarcy_incoming import get_hierarchy_incoming, to_prompt
from git_utils import checkout_commit, clone_repo, parse_diff

from client import APIClient
from models import (
    FilePosition,
    Position,
)

API_BASE_URL = "http://localhost:4444/v1"

logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")


def run_docker(workspace_path: str) -> subprocess.Popen:
    """Run the lsproxy in a Docker container.

    Args:
        workspace_path: Path to the workspace directory to be mounted in the container

    Raises:
        RuntimeError: If the server fails to start within 30 seconds

    Returns:
        subprocess.Popen: The subprocess running the Docker container
    """
    logging.info("Starting Docker container")
    with open("lsproxy.log", "w") as f:
        process = subprocess.Popen(
            ["./scripts/run.sh", workspace_path],
            stdout=f,
            stderr=subprocess.STDOUT,
            cwd="../../",
        )

    logging.info("Waiting for server to start...")
    for _ in range(120):  # Try for 120 seconds
        try:
            requests.get(f"{API_BASE_URL}/workspace/list-files")
            logging.info("Server is running")
            return process
        except requests.exceptions.ConnectionError:
            sleep(1)
    process.terminate()
    raise RuntimeError("Server failed to start after 120 seconds")


def main():
    parser = argparse.ArgumentParser(
        description="Calculate blast radius of code changes between two commits."
    )
    parser.add_argument("repo_url", help="URL of the git repository")
    parser.add_argument("commit1", help="First commit hash")
    parser.add_argument("commit2", help="Second commit hash")
    args = parser.parse_args()

    client = APIClient()
    all_nodes = set()
    all_edges = set()
    process = None  # Initialize process variable
    try:
        with tempfile.TemporaryDirectory() as temp_dir:
            clone_repo(args.repo_url, temp_dir)
            checkout_commit(temp_dir, args.commit1)
            process = run_docker(temp_dir)
            affected_lines, diff_text = parse_diff(temp_dir, args.commit1, args.commit2)
            affected_files = list(affected_lines.keys())
            lsp_files = client.list_files()
            affected_files = [file for file in affected_files if file in lsp_files]

            for file in affected_files:
                starting_positions = [
                    FilePosition(path=file, position=Position(line=line, character=0))
                    for line in affected_lines[file]
                ]
                nodes, edges = get_hierarchy_incoming(client, starting_positions)
                all_nodes.update(nodes)
                all_edges.update(edges)

        # Draw graph using networkx and matplotlib
        import networkx as nx
        import matplotlib.pyplot as plt

        G = nx.DiGraph()
        G.add_nodes_from(all_nodes)
        G.add_edges_from(all_edges)

        plt.figure(figsize=(12, 8))

        # Use hierarchical layout based on topological sort
        try:
            # Assign layers based on topological sort
            layers = {}
            blast_radius_input_str = (
                f"# Diff of the change\n```\n{diff_text}\n```\n# Call hierarchy \n"
            )
            for node in nx.topological_sort(G):
                blast_radius_input_str += to_prompt(node.source_code_context)
                preds = list(G.predecessors(node))
                layers[node] = max([layers[pred] for pred in preds], default=-1) + 1
            pos = {node: (i, layers[node]) for i, node in enumerate(layers)}

            with open("blast_radius_input.md", "w") as f:
                f.write(blast_radius_input_str)

            nx.draw(
                G,
                pos,
                with_labels=True,
                node_color="lightblue",
                node_size=150,
                arrowsize=20,
                font_size=6,
                edge_color="gray",
                width=1,
                arrows=True,
                arrowstyle="->",
            )

            plt.title(
                f"Blast Radius Graph\n(Nodes: {len(all_nodes)}, Edges: {len(all_edges)})"
            )

        except nx.NetworkXUnfeasible:
            logging.warning("Graph contains cycles, falling back to spring layout")
            pos = nx.spring_layout(G)
            nx.draw(
                G,
                pos,
                with_labels=True,
                node_color="lightblue",
                node_size=1500,
                arrowsize=20,
                font_size=8,
            )
            plt.title(
                f"Blast Radius Graph (Contains Cycles)\n(Nodes: {len(all_nodes)}, Edges: {len(all_edges)})"
            )

        plt.savefig("blast_radius_graph.png")
        plt.close()

        logging.info(
            f"Graph saved with {len(all_nodes)} nodes and {len(all_edges)} edges"
        )
    except Exception as e:
        logging.error(f"An error occurred: {e}")
        import traceback

        traceback.print_exc()
    finally:
        if process:
            logging.info("Shutting down Docker container")
            process.terminate()
            process.wait()
        # Force remove the temporary directory if it's not empty
        shutil.rmtree(temp_dir, ignore_errors=True)


if __name__ == "__main__":
    main()
