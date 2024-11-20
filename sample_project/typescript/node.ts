export class Node {
    constructor(
        public parent: Node | null,
        public x: number,
        public y: number,
        public g: number,
        public h: number
    ) {}

    // Arrow function for the getter
    f = (): number => this.g + this.h;

    // Alternative method using arrow function
    toString = (): string => `Node(${this.x}, ${this.y})`;
}
