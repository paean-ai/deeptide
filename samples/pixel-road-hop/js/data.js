// Pixel Road Hop - endless lane-crossing game (Crossy-Road style).

const VW = 360, VH = 480;
const TILE = 40;
const COLS = 9;
const HOP_DUR = 0.11;          // seconds per hop
const CREEP = 0.5;             // rows/sec the camera creeps forward when idle
const SAFE_ROWS = 4;           // first rows are guaranteed grass

const CAR_COLORS = ['#e8554f', '#4a8fe8', '#f2cf3f', '#ef9b3e', '#9a6cd8', '#5fc06e'];

// world row -> screen Y (top of the tile band)
function rowScreenY(worldRow, camRow) {
  return VH - 96 - (worldRow - camRow) * TILE;
}
