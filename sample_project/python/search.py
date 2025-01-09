from typing import Dict, Set, Tuple, List
from graph import AStarGraph
from decorators import log_execution_time

@log_execution_time
def initialize_search(start, end, graph: AStarGraph):
    G = {start: 0}  # Actual movement cost to each position from the start position
    F = {
        start: graph.heuristic(start, end)
    }  # Estimated movement cost of start to end going via this position
    closed_vertices = set()
    open_vertices = set([start])
    came_from = {}
    return G, F, closed_vertices, open_vertices, came_from

@log_execution_time
def a_star_search(start, end, graph: AStarGraph):
    def reconstruct_path(current: any, came_from: Dict) -> List:
        """
        Reconstructs the path from end to start using the came_from dictionary.
        Returns the path in correct order (start to end).
        """
        path = []
        while current in came_from:
            path.append(current)
            current = came_from[current]
        path.append(start)
        return path[::-1]

    G, F, closed_vertices, open_vertices, came_from = initialize_search(
        start, end, graph
    )

    while open_vertices:
        current = min(open_vertices, key=lambda pos: F[pos])
        if current == end:
            return reconstruct_path(current, came_from), F[end]

        open_vertices.remove(current)
        closed_vertices.add(current)

        for neighbour in graph.get_vertex_neighbours(current):
            if neighbour in closed_vertices:
                continue
            
            candidate_g = G[current] + graph.move_cost(current, neighbour)
            
            if neighbour not in open_vertices:
                open_vertices.add(neighbour)
            elif candidate_g >= G.get(neighbour, float("inf")):
                continue
                
            came_from[neighbour] = current
            G[neighbour] = candidate_g
            F[neighbour] = G[neighbour] + graph.heuristic(neighbour, end)

    raise RuntimeError("A* failed to find a solution")
