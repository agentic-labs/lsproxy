<?php
namespace AStar;

/**
 * A* pathfinding implementation
 */
class AStar {
    private array $open = [];
    private array $closed = [];
    private array $path = [];
    private array $maze;
    private Node $now;
    private int $xstart;
    private int $ystart;
    private int $xend;
    private int $yend;
    private bool $diag;

    public function __construct(array $maze, int $xstart, int $ystart, bool $diag) {
        $this->maze = $maze;
        $this->xstart = $xstart;
        $this->ystart = $ystart;
        $this->diag = $diag;
        $this->now = new Node(null, $xstart, $ystart, 0, 0);
    }

    public function findPathTo(int $xend, int $yend): ?array {
        $this->xend = $xend;
        $this->yend = $yend;
        $this->closed[] = $this->now;
        $this->addNeighborsToOpenList();

        while ($this->now->x !== $this->xend || $this->now->y !== $this->yend) {
            if (empty($this->open)) {
                return null;
            }
            $this->now = array_shift($this->open);
            $this->closed[] = $this->now;
            $this->addNeighborsToOpenList();
        }

        array_unshift($this->path, $this->now);
        while ($this->now->x !== $this->xstart || $this->now->y !== $this->ystart) {
            $this->now = $this->now->parent;
            array_unshift($this->path, $this->now);
        }

        return $this->path;
    }

    private function addNeighborsToOpenList(): void {
        for ($x = -1; $x <= 1; $x++) {
            for ($y = -1; $y <= 1; $y++) {
                if (!$this->diag && $x !== 0 && $y !== 0) {
                    continue;
                }

                $node = new Node(
                    $this->now,
                    $this->now->x + $x,
                    $this->now->y + $y,
                    $this->now->g,
                    $this->distance($this->now->x + $x, $this->now->y + $y)
                );

                if (($x !== 0 || $y !== 0) &&
                    $this->now->x + $x >= 0 && $this->now->x + $x < count($this->maze[0]) &&
                    $this->now->y + $y >= 0 && $this->now->y + $y < count($this->maze) &&
                    $this->maze[$this->now->y + $y][$this->now->x + $x] !== -1 &&
                    !$this->findNeighborInList($this->open, $node) &&
                    !$this->findNeighborInList($this->closed, $node)) {

                    $node->g = $node->parent->g + 1.0;
                    $node->g += $this->maze[$this->now->y + $y][$this->now->x + $x];
                    $this->open[] = $node;
                }
            }
        }

        usort($this->open, fn($a, $b) => $a->compareTo($b));
    }

    private function distance(int $x, int $y): float {
        return sqrt(pow($x - $this->xend, 2) + pow($y - $this->yend, 2));
    }

    private function findNeighborInList(array $list, Node $node): bool {
        foreach ($list as $n) {
            if ($n->x === $node->x && $n->y === $node->y) {
                return true;
            }
        }
        return false;
    }
}
