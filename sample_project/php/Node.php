<?php
namespace AStar;

class Node implements Comparable {
    public ?Node $parent;
    public int $x;
    public int $y;
    public float $g;
    public float $h;

    public function __construct(?Node $parent, int $x, int $y, float $g, float $h) {
        $this->parent = $parent;
        $this->x = $x;
        $this->y = $y;
        $this->g = $g;
        $this->h = $h;
    }

    public function compareTo($other): int {
        $thisF = $this->g + $this->h;
        $otherF = $other->g + $other->h;
        return $thisF <=> $otherF;
    }
}
