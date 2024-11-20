import { Node } from './node';
import { Grid, Point } from './types';

// Interface definition
interface IAStarPathfinder {
    findPathTo(xend: number, yend: number): Node[] | null;
}

export class AStar implements IAStarPathfinder {
    private open: Node[] = [];
    private closed: Node[] = [];
    private path: Node[] = [];
    private now: Node;

    constructor(
        private maze: Grid,
        private xstart: number,
        private ystart: number,
        private diag: boolean
    ) {
        this.now = new Node(null, xstart, ystart, 0, 0);
    }

    // Arrow function method implementing interface method
    findPathTo = (xend: number, yend: number): Node[] | null => {
        this.closed.push(this.now);
        this.addNeighborsToOpenList(xend, yend);
        while (this.now.x !== xend || this.now.y !== yend) {
            if (this.open.length === 0) {
                return null;
            }
            this.now = this.open[0];
            this.open.splice(0, 1);
            this.closed.push(this.now);
            this.addNeighborsToOpenList(xend, yend);
        }
        this.path = [this.now];
        while (this.now.x !== this.xstart || this.now.y !== this.ystart) {
            this.now = this.now.parent!;
            this.path.unshift(this.now);
        }
        return this.path;
    };

    // Traditional method with function keyword
    private isInBounds(point: Point): boolean {
        return (
            point.x >= 0 && 
            point.x < this.maze[0].length &&
            point.y >= 0 && 
            point.y < this.maze.length
        );
    }

    // Arrow function as property
    private isWalkable = (point: Point): boolean => {
        return this.maze[point.y][point.x] !== -1;
    };

    // Method shorthand notation
    private addNeighborsToOpenList(xend: number, yend: number): void {
        for (let x = -1; x <= 1; x++) {
            for (let y = -1; y <= 1; y++) {
                if (!this.diag && x !== 0 && y !== 0) {
                    continue;
                }
                const newPoint = {
                    x: this.now.x + x,
                    y: this.now.y + y
                };
                if (x === 0 && y === 0) continue;
                if (!this.isInBounds(newPoint)) continue;
                if (!this.isWalkable(newPoint)) continue;
                const node = new Node(
                    this.now,
                    newPoint.x,
                    newPoint.y,
                    this.now.g,
                    this.distance(newPoint.x, newPoint.y, xend, yend)
                );
                if (
                    this.findNeighborInList(this.open, node) ||
                    this.findNeighborInList(this.closed, node)
                ) continue;
                node.g = node.parent!.g + 1;
                node.g += this.maze[newPoint.y][newPoint.x];
                this.open.push(node);
            }
        }
        this.open.sort((a, b) => a.f() - b.f());
    }

    // Arrow function with explicit parameter types
    private distance = (x1: number, y1: number, x2: number, y2: number): number => {
        return Math.sqrt(Math.pow(x2 - x1, 2) + Math.pow(y2 - y1, 2));
    };

    // Traditional function expression assigned to property
    private findNeighborInList = function(list: Node[], node: Node): boolean {
        return list.some(n => n.x === node.x && n.y === node.y);
    };
}
