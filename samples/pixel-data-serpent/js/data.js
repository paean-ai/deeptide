// Pixel Data Serpent - content data: grid, food, sector tuning

const GRID = 20;             // cells per side
const CW = 480;              // canvas size (square)
const CELL = CW / GRID;

const GOAL_PER_SECTOR = 6;   // nodes to collect to advance a sector

// Step interval (seconds) shrinks each sector for rising speed.
function stepInterval(sector) {
  return Math.max(0.072, 0.165 - sector * 0.011);
}
// Obstacle (firewall) count placed at the start of each sector.
function firewallCount(sector) {
  return Math.min(34, 1 + sector * 2);
}

// Food kinds. `normal` always; specials roll in from sector 3.
const FOOD = {
  normal: { color: '#5fd9c0', score: 10, grow: 1 },
  golden: { color: '#f4c85a', score: 60, grow: 2 },
  shrink: { color: '#7aa0ff', score: 10, grow: -3 },
  slow:   { color: '#b87ae0', score: 10, grow: 1 },
};

function rollFoodKind(sector) {
  if (sector < 3) return 'normal';
  const r = Math.random();
  if (r < 0.10) return 'golden';
  if (r < 0.17) return 'shrink';
  if (r < 0.24) return 'slow';
  return 'normal';
}
