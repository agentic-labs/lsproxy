package golang_astar

// Grid represents the search space with barriers
type Grid struct {
	Width    int
	Height   int
	Barriers map[Node]bool
}

// NewGrid creates a new grid with the given dimensions
func NewGrid(width, height int) *Grid {
	return &Grid{
		Width:    width,
		Height:   height,
		Barriers: make(map[Node]bool),
	}
}

// IsValidPosition checks if a position is within grid bounds
func (g *Grid) IsValidPosition(n Node) bool {
	return n.X >= 0 && n.X < g.Width && n.Y >= 0 && n.Y < g.Height
}

// GetNeighbors returns valid neighboring nodes
func (g *Grid) GetNeighbors(n Node) []Arc {
	neighbors := make([]Arc, 0, 8)

	// Check all 8 adjacent positions
	for dx := -1; dx <= 1; dx++ {
		for dy := -1; dy <= 1; dy++ {
			if dx == 0 && dy == 0 {
				continue
			}

			next := Node{n.X + dx, n.Y + dy}
			if !g.IsValidPosition(next) {
				continue
			}

			cost := Cost(1)
			if g.Barriers[next] {
				cost = 100
			}
			neighbors = append(neighbors, Arc{next, cost})
		}
	}
	return neighbors
}
