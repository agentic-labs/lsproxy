import { AStar } from './astar';
import { PathVisualizer } from './visualization';
import { Grid } from './types';

// Arrow function for main
const main = (): void => {
    const maze: Grid = [
        [0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 100, 100, 100, 0, 0],
        [0, 0, 0, 0, 0, 100, 0, 0],
        [0, 0, 100, 0, 0, 100, 0, 0],
        [0, 0, 100, 0, 0, 100, 0, 0],
        [0, 0, 100, 100, 100, 100, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0],
    ];

    const astar = new AStar(maze, 0, 0, true);
    const path = astar.findPathTo(7, 7);

    if (path) {
        PathVisualizer.printPath(path);
        PathVisualizer.visualizePath(maze, path);
    } else {
        console.log('No path found!');
    }
};

// IIFE (Immediately Invoked Function Expression) with arrow function
(() => {
    main();
})();

export { main };
