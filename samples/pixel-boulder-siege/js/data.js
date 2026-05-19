// Pixel Boulder Siege - layout constants, block types, fortress generation.

const VW = 360, VH = 480;

// Block grid: 12 columns wide, anchored so the bottom row rests on the ground.
const B = 22;            // block size in pixels
const COLS = 12;
const ROWS = 11;
const GROUND_Y = 430;
const ORIGIN_X = VW - COLS * B; // 96 — structures fill the right side

// Cannon sits on the left, firing into the fortress zone.
const CANNON_X = 44;
const CANNON_Y = GROUND_Y - 16;

// Projectile launch tuning.
const GRAVITY = 760;
const MIN_SPEED = 168;
const MAX_SPEED = 560;
const MAX_PULL = 150;    // drag distance (px) for full power

// Block archetypes: hp and how bouncy a boulder is off them.
const BLOCKS = {
  W: { hp: 26,  name: 'wood',  bounce: 0.34 },
  S: { hp: 84,  name: 'stone', bounce: 0.30 },
  G: { hp: 9,   name: 'glass', bounce: 0.52 },
};

function cellTop(r) { return GROUND_Y - (ROWS - r) * B; }
function cellX(col) { return ORIGIN_X + col * B; }

// Procedurally build a fortress for the given round: a set of block towers
// with goblins perched on top, embedded inside, or standing on open ground.
// Goblins are never sealed under stone, so every round stays winnable.
function generateFortress(round) {
  const ents = [];
  const usedCols = new Set();
  const goblinTarget = Math.min(7, 3 + Math.floor(round / 2));
  const stoneChance = Math.min(0.46, 0.1 + round * 0.04);
  const towerCount = 3 + Math.floor(Math.random() * 3);
  let goblins = 0;

  for (let i = 0; i < towerCount; i++) {
    const col = 1 + Math.floor(Math.random() * (COLS - 2));
    if (usedCols.has(col)) continue;
    usedCols.add(col);
    const h = Math.min(ROWS - 3, 2 + Math.floor(Math.random() * (3 + Math.min(4, round))));
    const stack = [];
    for (let k = 0; k < h; k++) {
      const type = Math.random() < stoneChance ? 'S'
        : Math.random() < 0.32 ? 'G' : 'W';
      stack.push({ col, r: ROWS - 1 - k, type });
    }
    if (goblins < goblinTarget && Math.random() < 0.82) {
      if (Math.random() < 0.5) {
        stack.push({ col, r: ROWS - 1 - h, type: 'X' }); // perched on top
      } else {
        const idx = 1 + Math.floor(Math.random() * (stack.length - 1));
        stack[idx] = { col, r: stack[idx].r, type: 'X' }; // embedded
      }
      goblins++;
    }
    ents.push(...stack);
  }

  // Fill the remaining goblins as lone defenders on open ground.
  let guard = 0;
  while (goblins < goblinTarget && guard < 300) {
    guard++;
    const col = Math.floor(Math.random() * COLS);
    if (usedCols.has(col)) continue;
    usedCols.add(col);
    ents.push({ col, r: ROWS - 1, type: 'X' });
    goblins++;
  }

  return { entities: ents, goblinCount: goblins, shots: goblins + 2 };
}
