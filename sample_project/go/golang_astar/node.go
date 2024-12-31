package golang_astar

import "fmt"

// Node represents a position in the grid
type Node struct {
	X, Y int
}

// String provides a string representation of Node
func (n Node) String() string {
	return fmt.Sprintf("(%d,%d)", n.X, n.Y)
}

// Equal checks if two nodes have the same coordinates
func (n Node) Equal(other Node) bool {
	return n.X == other.X && n.Y == other.Y
}

// Cost represents the cost to move between nodes
type Cost int

// Arc represents a connection between nodes with an associated cost
type Arc struct {
	To   Node
	Cost Cost
}

// Heuristic calculates estimated cost to reach another node
func (n Node) Heuristic(from Node) int {
	dx := n.X - from.X
	if dx < 0 {
		dx = -dx
	}
	dy := n.Y - from.Y
	if dy < 0 {
		dy = -dy
	}
	if dx > dy {
		return dx
	}
	return dy
}

// To returns list of arcs from this node to neighbors
func (n Node) To() []Arc {
	neighbors := make([]Arc, 0, 8)
	for dx := -1; dx <= 1; dx++ {
		for dy := -1; dy <= 1; dy++ {
			if dx == 0 && dy == 0 {
				continue
			}
			next := Node{n.X + dx, n.Y + dy}
			neighbors = append(neighbors, Arc{To: next, Cost: 1})
		}
	}
	return neighbors
}
