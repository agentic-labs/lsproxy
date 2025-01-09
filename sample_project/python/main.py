import matplotlib.pyplot as plt
from graph import AStarGraph
from search import a_star_search
from decorators import log_execution_time

@log_execution_time
def plot_path(path, graph: AStarGraph) -> None:
    plt.plot([v[0] for v in path], [v[1] for v in path])
    for barrier in graph.barriers:
        plt.plot([v[0] for v in barrier], [v[1] for v in barrier])
    plt.xlim(-1, 8)
    plt.ylim(-1, 8)
    plt.show()

def main():
    graph = AStarGraph()
    result, cost = a_star_search((0, 0), (7, 7), graph)
    print("route", result)
    print("cost", cost)
    plot_path(result, graph)

if __name__ == "__main__":
    main()
