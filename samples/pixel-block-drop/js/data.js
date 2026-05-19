// Pixel Block Drop - falling-block puzzle data.

const VW = 300, VH = 440;
const COLS = 10, ROWS = 20, CELL = 14;
const BX = 12, BY = 30;          // board pixel origin
const SPAWN_X = 3, SPAWN_Y = 0;

// Seven tetrominoes, each with 4 rotation states. A state is 4 [x,y] cells
// inside a small box; the spawn box top-left lands at (piece.x, piece.y).
const PIECES = {
  I: { color: '#4ad6e0', states: [
    [[0,1],[1,1],[2,1],[3,1]], [[2,0],[2,1],[2,2],[2,3]],
    [[0,2],[1,2],[2,2],[3,2]], [[1,0],[1,1],[1,2],[1,3]] ] },
  O: { color: '#f2cf3f', states: [
    [[1,0],[2,0],[1,1],[2,1]], [[1,0],[2,0],[1,1],[2,1]],
    [[1,0],[2,0],[1,1],[2,1]], [[1,0],[2,0],[1,1],[2,1]] ] },
  T: { color: '#a86cd8', states: [
    [[1,0],[0,1],[1,1],[2,1]], [[1,0],[1,1],[2,1],[1,2]],
    [[0,1],[1,1],[2,1],[1,2]], [[1,0],[0,1],[1,1],[1,2]] ] },
  S: { color: '#5fc06e', states: [
    [[1,0],[2,0],[0,1],[1,1]], [[1,0],[1,1],[2,1],[2,2]],
    [[1,1],[2,1],[0,2],[1,2]], [[0,0],[0,1],[1,1],[1,2]] ] },
  Z: { color: '#e8554f', states: [
    [[0,0],[1,0],[1,1],[2,1]], [[2,0],[1,1],[2,1],[1,2]],
    [[0,1],[1,1],[1,2],[2,2]], [[1,0],[0,1],[1,1],[0,2]] ] },
  J: { color: '#4a8fe8', states: [
    [[0,0],[0,1],[1,1],[2,1]], [[1,0],[2,0],[1,1],[1,2]],
    [[0,1],[1,1],[2,1],[2,2]], [[1,0],[1,1],[0,2],[1,2]] ] },
  L: { color: '#ef9b3e', states: [
    [[2,0],[0,1],[1,1],[2,1]], [[1,0],[1,1],[1,2],[2,2]],
    [[0,1],[1,1],[2,1],[0,2]], [[0,0],[1,0],[1,1],[1,2]] ] },
};
const PIECE_IDS = ['I', 'O', 'T', 'S', 'Z', 'J', 'L'];

const LINE_SCORES = [0, 100, 300, 500, 800];
const LOCK_DELAY = 0.5;            // seconds a resting piece waits before locking
const MAX_LOCK_RESETS = 15;
const KICKS = [0, -1, 1, -2, 2];   // wall-kick x offsets tried on rotation

// fall interval (seconds per row) by level
function gravitySec(level) {
  return Math.max(0.05, 0.85 - level * 0.062);
}
