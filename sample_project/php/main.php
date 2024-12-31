<?php
namespace AStar;

require_once __DIR__ . '/Comparable.php';
require_once __DIR__ . '/Node.php';
require_once __DIR__ . '/AStar.php';

// -1 = blocked
// 0+ = additional movement cost
$maze = [
    [0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 100, 100, 100, 0, 0],
    [0, 0, 0, 0, 0, 100, 0, 0],
    [0, 0, 100, 0, 0, 100, 0, 0],
    [0, 0, 100, 0, 0, 100, 0, 0],
    [0, 0, 100, 100, 100, 100, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0],
];

$astar = new AStar($maze, 0, 0, true);
$path = $astar->findPathTo(7, 7);

if ($path !== null) {
    foreach ($path as $node) {
        echo "[{$node->x}, {$node->y}] ";
        $maze[$node->y][$node->x] = -1;
    }
    echo "\nTotal cost: " . number_format(end($path)->g, 2) . "\n";

    foreach ($maze as $row) {
        foreach ($row as $cell) {
            switch ($cell) {
                case 0:
                    echo "_";
                    break;
                case -1:
                    echo "*";
                    break;
                default:
                    echo "#";
            }
        }
        echo "\n";
    }
}
