package golang_astar

import (
	"container/heap"
)

// node represents a node in the search path
type searchNode struct {
	pos     Node
	parent  *searchNode
	g, h, f Cost
	index   int // for heap.Interface
}

// nodeHeap implements heap.Interface
type nodeHeap []*searchNode

func (h nodeHeap) Len() int           { return len(h) }
func (h nodeHeap) Less(i, j int) bool { return h[i].f < h[j].f }
func (h nodeHeap) Swap(i, j int) {
	h[i], h[j] = h[j], h[i]
	h[i].index = i
	h[j].index = j
}
func (h *nodeHeap) Push(x interface{}) {
	n := len(*h)
	item := x.(*searchNode)
	item.index = n
	*h = append(*h, item)
}
func (h *nodeHeap) Pop() interface{} {
	old := *h
	n := len(old)
	item := old[n-1]
	old[n-1] = nil
	item.index = -1
	*h = old[0 : n-1]
	return item
}

// Heuristic estimates remaining cost to goal
func Heuristic(current, goal Node) Cost {
	dx := current.X - goal.X
	if dx < 0 {
		dx = -dx
	}
	dy := current.Y - goal.Y
	if dy < 0 {
		dy = -dy
	}
	if dx > dy {
		return Cost(dx)
	}
	return Cost(dy)
}

// FindPath finds the shortest path between start and goal
func FindPath(grid *Grid, start, goal Node) ([]Node, Cost) {
	openSet := &nodeHeap{}
	heap.Init(openSet)

	startNode := &searchNode{
		pos:    start,
		g:      0,
		h:      Heuristic(start, goal),
		parent: nil,
	}
	startNode.f = startNode.g + startNode.h
	heap.Push(openSet, startNode)

	closedSet := make(map[Node]*searchNode)

	for openSet.Len() > 0 {
		current := heap.Pop(openSet).(*searchNode)

		if current.pos.Equal(goal) {
			// Reconstruct path
			path := []Node{}
			cost := current.g
			for current != nil {
				path = append([]Node{current.pos}, path...)
				current = current.parent
			}
			return path, cost
		}

		closedSet[current.pos] = current

		for _, arc := range grid.GetNeighbors(current.pos) {
			if _, exists := closedSet[arc.To]; exists {
				continue
			}

			g := current.g + arc.Cost

			var neighbor *searchNode
			for _, node := range *openSet {
				if node.pos.Equal(arc.To) {
					neighbor = node
					break
				}
			}

			if neighbor == nil {
				neighbor = &searchNode{
					pos:    arc.To,
					parent: current,
					g:      g,
					h:      Heuristic(arc.To, goal),
				}
				neighbor.f = neighbor.g + neighbor.h
				heap.Push(openSet, neighbor)
			} else if g < neighbor.g {
				neighbor.parent = current
				neighbor.g = g
				neighbor.f = g + neighbor.h
				heap.Fix(openSet, neighbor.index)
			}
		}
	}

	return nil, 0 // No path found
}
