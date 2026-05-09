class Camera {
    constructor(worldWidth, worldHeight, screenWidth, screenHeight) {
        this.worldWidth = worldWidth;
        this.worldHeight = worldHeight;
        this.screenWidth = screenWidth;
        this.screenHeight = screenHeight;
        this.x = 0;
        this.y = 0;
    }

    follow(target) {
        this.x = target.x - this.screenWidth / 2;
        this.y = target.y - this.screenHeight / 2;
        this.x = Math.max(0, Math.min(this.x, this.worldWidth - this.screenWidth));
        this.y = Math.max(0, Math.min(this.y, this.worldHeight - this.screenHeight));
    }

    worldToScreen(wx, wy) {
        return { x: wx - this.x, y: wy - this.y };
    }

    isVisible(wx, wy, margin = 80) {
        return wx + margin > this.x && wx - margin < this.x + this.screenWidth &&
               wy + margin > this.y && wy - margin < this.y + this.screenHeight;
    }

    screenToWorld(sx, sy) {
        return { x: sx + this.x, y: sy + this.y };
    }

    get worldLeft() { return this.x; }
    get worldRight() { return this.x + this.screenWidth; }
    get worldTop() { return this.y; }
    get worldBottom() { return this.y + this.screenHeight; }
}
