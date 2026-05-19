// Pixel Orb Fusion - a 2048-style merge puzzle. Slide all orbs; equal orbs
// fuse into the next tier. Reach the 2048 orb, then chase a high score.

const GRID = 4;
const CELL = 96;
const BOARD = GRID * CELL;     // 384
const WIN_TIER = 11;           // 2^11 = 2048

function orbValue(tier) { return Math.pow(2, tier); }

// Hue ramp so every tier reads as a distinct orb colour.
function orbHue(tier) { return (tier * 33 + 6) % 360; }

// Lines of cells (destination-first) for each swipe direction.
// dir: 'left' | 'right' | 'up' | 'down'
function linesFor(dir) {
  const lines = [];
  for (let i = 0; i < GRID; i++) {
    const line = [];
    for (let j = 0; j < GRID; j++) {
      if (dir === 'left')  line.push({ r: i, c: j });
      if (dir === 'right') line.push({ r: i, c: GRID - 1 - j });
      if (dir === 'up')    line.push({ r: j, c: i });
      if (dir === 'down')  line.push({ r: GRID - 1 - j, c: i });
    }
    lines.push(line);
  }
  return lines;
}
