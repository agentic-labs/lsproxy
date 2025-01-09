from typing import List, Tuple
from decorators import log_execution_time
from enum import Enum

class GraphBase:
    pass

class CostStrategy(Enum):
    BARRIER = "barrier"
    DISTANCE = "distance"
    COMBINED = "combined"

class AStarGraph(GraphBase):
    def __init__(self):
        self._barriers: List[List[Tuple[int, int]]] = []
        self._barriers.append([
            (2, 4), (2, 5), (2, 6),
            (3, 6), (4, 6), (5, 6),
            (5, 5), (5, 4), (5, 3),
            (5, 2), (4, 2), (3, 2),
        ])

    @property
    def barriers(self):
        return self._barriers

    def _barrier_cost(self, a: Tuple[int, int], b: Tuple[int, int]) -> float:
        """Original barrier-based cost calculation"""
        for barrier in self.barriers:
            if b in barrier:
                return 100
        return 1

    def _distance_cost(self, a: Tuple[int, int], b: Tuple[int, int]) -> float:
        """Cost based on Manhattan distance between points"""
        return abs(b[0] - a[0]) + abs(b[1] - a[1])

    def _combined_cost(self, a: Tuple[int, int], b: Tuple[int, int]) -> float:
        """Combines barrier and distance costs"""
        barrier_cost = self._barrier_cost(a, b)
        distance_cost = self._distance_cost(a, b)
        return barrier_cost * distance_cost

    def move_cost(self, a: Tuple[int, int], b: Tuple[int, int], 
                 strategy: CostStrategy = CostStrategy.BARRIER) -> float:
        """
        Calculate movement cost between two points using specified strategy.
        
        Args:
            a: Starting position
            b: Ending position
            strategy: Cost calculation strategy to use
            
        Returns:
            float: Cost of movement
        """
        if strategy == CostStrategy.BARRIER:
            cost_function = self._barrier_cost
        elif strategy == CostStrategy.DISTANCE:
            cost_function = self._distance_cost
        elif strategy == CostStrategy.COMBINED:
            cost_function = self._combined_cost
        else:
            raise ValueError(f"Unknown cost strategy: {strategy}")
        
        return cost_function(a, b)

    @log_execution_time
    def heuristic(self, start, goal):
        D = 1
        D2 = 1
        dx = abs(start[0] - goal[0])
        dy = abs(start[1] - goal[1])
        return D * (dx + dy) + (D2 - 2 * D) * min(dx, dy)

    @log_execution_time
    def get_vertex_neighbours(self, pos, cost_strategy: CostStrategy = CostStrategy.BARRIER):
        n = []
        for dx, dy in [
            (1, 0), (-1, 0), (0, 1), (0, -1),
            (1, 1), (-1, 1), (1, -1), (-1, -1),
        ]:
            x2 = pos[0] + dx
            y2 = pos[1] + dy
            if x2 < 0 or x2 > 7 or y2 < 0 or y2 > 7:
                continue
            if self.move_cost(pos, (x2, y2), strategy=cost_strategy) < 100:
                n.append((x2, y2))
        return n
