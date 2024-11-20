import React, { useState, useEffect } from 'react';
import { Node } from './node';
import { AStar } from './astar';
import { Grid } from './types';

interface PathfinderDisplayProps {
    initialMaze?: Grid;
    start?: { x: number; y: number };
    end?: { x: number; y: number };
    allowDiagonal?: boolean;
}

export const PathfinderDisplay: React.FC<PathfinderDisplayProps> = ({
    initialMaze = [
        [0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 100, 100, 100, 0, 0],
        [0, 0, 0, 0, 0, 100, 0, 0],
        [0, 0, 100, 0, 0, 100, 0, 0],
        [0, 0, 100, 0, 0, 100, 0, 0],
        [0, 0, 100, 100, 100, 100, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0],
    ],
    start = { x: 0, y: 0 },
    end = { x: 7, y: 7 },
    allowDiagonal = true,
}) => {
    const [maze, setMaze] = useState<Grid>(initialMaze);
    const [path, setPath] = useState<Node[] | null>(null);
    const [isAnimating, setIsAnimating] = useState(false);
    const [currentStep, setCurrentStep] = useState(0);

    const findPath = () => {
        const astar = new AStar(maze, start.x, start.y, allowDiagonal);
        const newPath = astar.findPathTo(end.x, end.y);
        setPath(newPath);
        setCurrentStep(0);
        setIsAnimating(true);
    };

    useEffect(() => {
        if (isAnimating && path && currentStep < path.length) {
            const timer = setTimeout(() => {
                setCurrentStep(prev => prev + 1);
            }, 500);
            return () => clearTimeout(timer);
        }
        if (currentStep >= (path?.length ?? 0)) {
            setIsAnimating(false);
        }
    }, [isAnimating, currentStep, path]);

    const getCellColor = (x: number, y: number): string => {
        if (x === start.x && y === start.y) return 'bg-green-500';
        if (x === end.x && y === end.y) return 'bg-red-500';
        if (path?.slice(0, currentStep + 1).some(node => node.x === x && node.y === y)) {
            return 'bg-blue-500';
        }
        if (maze[y][x] === 100) return 'bg-gray-500';
        if (maze[y][x] === -1) return 'bg-black';
        return 'bg-white';
    };

    const toggleCell = (x: number, y: number) => {
        if ((x === start.x && y === start.y) || (x === end.x && y === end.y)) return;
        
        const newMaze = maze.map(row => [...row]);
        newMaze[y][x] = newMaze[y][x] === 100 ? 0 : 100;
        setMaze(newMaze);
        setPath(null);
        setCurrentStep(0);
    };

    return (
        <div className="p-4">
            <div className="mb-4 space-x-2">
                <button
                    onClick={findPath}
                    className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
                    disabled={isAnimating}
                >
                    {isAnimating ? 'Finding Path...' : 'Find Path'}
                </button>
                <button
                    onClick={() => {
                        setMaze(initialMaze);
                        setPath(null);
                        setCurrentStep(0);
                    }}
                    className="px-4 py-2 bg-gray-500 text-white rounded hover:bg-gray-600"
                    disabled={isAnimating}
                >
                    Reset
                </button>
            </div>
            
            <div className="inline-block border border-gray-200">
                {maze.map((row, y) => (
                    <div key={y} className="flex">
                        {row.map((_, x) => (
                            <div
                                key={`${x}-${y}`}
                                className={`w-8 h-8 border border-gray-200 ${getCellColor(x, y)} 
                                    transition-colors duration-300 cursor-pointer`}
                                onClick={() => toggleCell(x, y)}
                            />
                        ))}
                    </div>
                ))}
            </div>

            <div className="mt-4">
                <div className="text-sm text-gray-600">
                    Click cells to toggle walls. Green = Start, Red = End, Blue = Path
                </div>
                {path && (
                    <div className="mt-2 text-sm">
                        Path length: {path.length} steps
                        <br />
                        Total cost: {path[path.length - 1].g.toFixed(2)}
                    </div>
                )}
            </div>
        </div>
    );
};
