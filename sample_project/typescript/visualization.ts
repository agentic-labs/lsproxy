import { Node } from './node';
import { Grid } from './types';

export class PathVisualizer {
    // Traditional static method
    static visualizePath(maze: Grid, path: Node[]): void {
        const visualMaze = maze.map(row => [...row]);
        
        // Arrow function in forEach
        path.forEach((node): void => {
            visualMaze[node.y][node.x] = -1;
        });

        console.log('\nPath visualization:');
        visualMaze.forEach(row => {
            console.log(row.map(cell => {
                if (cell === 0) return '_';
                if (cell === -1) return '*';
                return '#';
            }).join(''));
        });

        console.log(`\nTotal cost: ${path[path.length - 1].g.toFixed(2)}`);
    }

    // Arrow function static method
    static printPath = (path: Node[]): void => {
        console.log('Path coordinates:');
        path.forEach(node => {
            console.log(`[${node.x}, ${node.y}]`);
        });
    };
}
