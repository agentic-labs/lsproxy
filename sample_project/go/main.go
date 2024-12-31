package main

import (
	"astar_test/golang_astar"
	"fmt"
)

func main() {
	// Create 8x8 grid
	grid := golang_astar.NewGrid(8, 8)

	// Add barriers
	barriers := []golang_astar.Node{
		{2, 4}, {2, 5}, {2, 6}, {3, 6}, {4, 6}, {5, 6},
		{5, 5}, {5, 4}, {5, 3}, {5, 2}, {4, 2}, {3, 2},
	}

	for _, b := range barriers {
		grid.Barriers[b] = true
	}

	start := golang_astar.Node{0, 0}
	goal := golang_astar.Node{7, 7}

	fmt.Printf("Finding path from %v to %v\n", start, goal)

	path, cost := golang_astar.FindPath(grid, start, goal)
	if path == nil {
		fmt.Println("No path found!")
		return
	}

	fmt.Printf("Path found with cost %d:\n", cost)
	for _, node := range path {
		fmt.Printf("%v ", node)
	}
	fmt.Println()
}
