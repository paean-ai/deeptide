// Pixel Bloxorz - roll a 1x1x2 block on a tile grid into the hole.
//
// The block has three orientations:
//   'up'    standing on its end; footprint = one cell  (cx, cy)
//   'h'     lying horizontally;   footprint = two cells (cx, cy) + (cx+1, cy)
//   'v'     lying vertically;     footprint = two cells (cx, cy) + (cx, cy+1)
//
// Rolling rules (origin (cx, cy) marks the "anchor" — the bottom-left footprint cell):
//   from 'up' (1x1):
//     right -> 'h' at (cx+1, cy)     left -> 'h' at (cx-2, cy)
//     down  -> 'v' at (cx,   cy+1)   up   -> 'v' at (cx,   cy-2)
//   from 'h' (2x1):
//     right -> 'up' at (cx+2, cy)    left -> 'up' at (cx-1, cy)
//     down  -> 'h' at (cx,   cy+1)   up   -> 'h' at (cx,   cy-1)
//   from 'v' (1x2):
//     right -> 'v' at (cx+1, cy)     left -> 'v' at (cx-1, cy)
//     down  -> 'up' at (cx, cy+2)    up   -> 'up' at (cx, cy-1)
//
// Tile types:
//   '.'  solid tile
//   ' '  void (off-grid)
//   'w'  weak tile — supports a lying footprint but the block CANNOT stand
//        on it (standing weight is too concentrated); attempting to do so
//        falls through.
//   'G'  goal hole — only finishes when the block is STANDING exactly on
//        this cell.

const VW = 360, VH = 480;

// 6 hand-designed boards each verified solvable by an in-process BFS over
// (col, row, orient) states; `par` is the BFS minimum so 3 stars = an
// optimal solve. The full sweep + selection lives in `/tmp/probe_bx*.js`.
const LEVELS = [
  // 1. Sprout — gentle 5x3 intro, two rolls and you're standing on the goal.
  {
    name: ['Sprout', '萌芽'], par: 2,
    rows: [
      '.....',
      '.S..G',
      '.....',
    ],
  },
  // 2. Splinter — first taste of a weak tile; you must cross it while lying.
  {
    name: ['Splinter', '裂纹'], par: 4,
    rows: [
      '.....',
      '.S...',
      '..w..',
      '....G',
    ],
  },
  // 3. Crevice — a void carved through the middle row; detour around.
  {
    name: ['Crevice', '断垣'], par: 6,
    rows: [
      '.......',
      '.S.....',
      '....   ',
      '.......',
      '......G',
    ],
  },
  // 4. Bend — small L-shape; tight space + diagonal goal.
  {
    name: ['Bend', '弯角'], par: 7,
    rows: [
      '....',
      '.S..',
      '....',
      '...G',
    ],
  },
  // 5. Maze — the smallest board with the longest path: nine moves on a 4x3.
  {
    name: ['Maze', '迷踪'], par: 9,
    rows: [
      '....',
      '.S.G',
      '....',
    ],
  },
  // 6. Vault — a 4x3 with a weak tile that turns this into a ten-move
  // brain-twister.
  {
    name: ['Vault', '秘室'], par: 10,
    rows: [
      '....',
      '.S..',
      '.w.G',
    ],
  },
  // 7. Switchback — a bare 4x3 with one corner clipped; the goal sits a
  // tile away yet the only standing approach winds thirteen rolls.
  {
    name: ['Switchback', '回折'], par: 13,
    rows: [
      '....',
      '..S.',
      ' .G.',
    ],
  },
  // 8. Catacomb — a 5x4 vault: a weak tile and a void box the route in.
  {
    name: ['Catacomb', '地窟'], par: 16,
    rows: [
      '  w..',
      '..G..',
      '.....',
      '.S ..',
    ],
  },
  // 9. Keystone — finale: a long 6x3 gauntlet of weak tiles down the far
  // wall; nineteen rolls to set the block upright on the goal.
  {
    name: ['Keystone', '拱心石'], par: 19,
    rows: [
      '...w.G',
      '.....w',
      'S. ..w',
    ],
  },
];
const LEVEL_COUNT = LEVELS.length;

// ---- helpers -----------------------------------------------------------
function parseLevel(level) {
  const rows = level.rows;
  const h = rows.length;
  const w = rows.reduce((m, r) => Math.max(m, r.length), 0);
  const grid = [];
  let startCol = 0, startRow = 0, goalCol = 0, goalRow = 0;
  for (let r = 0; r < h; r++) {
    const row = [];
    for (let c = 0; c < w; c++) {
      let ch = rows[r][c] || ' ';
      if (ch === 'S') { startCol = c; startRow = r; ch = '.'; }
      else if (ch === 'G') { goalCol = c; goalRow = r; ch = 'G'; }
      row.push(ch);
    }
    grid.push(row);
  }
  return { w, h, grid, startCol, startRow, goalCol, goalRow };
}

function tileAt(grid, c, r) {
  if (r < 0 || r >= grid.length) return ' ';
  if (c < 0 || c >= grid[r].length) return ' ';
  return grid[r][c];
}

// Returns the footprint cells of the block in its current state.
function footprint(block) {
  if (block.orient === 'up') return [[block.col, block.row]];
  if (block.orient === 'h')  return [[block.col, block.row], [block.col + 1, block.row]];
  return [[block.col, block.row], [block.col, block.row + 1]];
}

// Apply a direction and return the new block state without checking support.
// dir: 'left' | 'right' | 'up' | 'down'
function rolled(block, dir) {
  const o = block.orient;
  const c = block.col, r = block.row;
  if (o === 'up') {
    if (dir === 'right') return { col: c + 1, row: r,     orient: 'h' };
    if (dir === 'left')  return { col: c - 2, row: r,     orient: 'h' };
    if (dir === 'down')  return { col: c,     row: r + 1, orient: 'v' };
    if (dir === 'up')    return { col: c,     row: r - 2, orient: 'v' };
  }
  if (o === 'h') {
    if (dir === 'right') return { col: c + 2, row: r,     orient: 'up' };
    if (dir === 'left')  return { col: c - 1, row: r,     orient: 'up' };
    if (dir === 'down')  return { col: c,     row: r + 1, orient: 'h' };
    if (dir === 'up')    return { col: c,     row: r - 1, orient: 'h' };
  }
  if (o === 'v') {
    if (dir === 'right') return { col: c + 1, row: r,     orient: 'v' };
    if (dir === 'left')  return { col: c - 1, row: r,     orient: 'v' };
    if (dir === 'down')  return { col: c,     row: r + 2, orient: 'up' };
    if (dir === 'up')    return { col: c,     row: r - 1, orient: 'up' };
  }
  return block;
}

// True iff EVERY footprint cell is supported by the grid, with the extra
// rule that weak tiles ('w') cannot hold a standing block.
function supported(block, grid) {
  const fp = footprint(block);
  for (const [c, r] of fp) {
    const t = tileAt(grid, c, r);
    if (t === ' ') return false;             // off-grid / void
    if (t === 'w' && block.orient === 'up') return false;   // weak crushes
  }
  return true;
}

// ---- runtime state -----------------------------------------------------
function buildGame(levelIndex) {
  const lv = LEVELS[levelIndex];
  const p = parseLevel(lv);
  const block = { col: p.startCol, row: p.startRow, orient: 'up' };
  return {
    levelIndex, lv,
    grid: p.grid, w: p.w, h: p.h,
    goal: { col: p.goalCol, row: p.goalRow },
    block,
    moves: 0,
    history: [],
    over: false, won: false, fell: false,
    flash: 0,
  };
}

function tryMove(s, dir) {
  if (s.over) return false;
  const next = rolled(s.block, dir);
  if (!next) return false;
  s.history.push({ block: { ...s.block } });
  s.block = next;
  s.moves++;
  if (!supported(s.block, s.grid)) {
    // The block fell — game over for this attempt; player can undo or restart.
    s.over = true; s.won = false; s.fell = true;
    s.flash = 0.5;
    return true;
  }
  // Win when the block is standing exactly on the goal tile.
  if (s.block.orient === 'up' && s.block.col === s.goal.col && s.block.row === s.goal.row) {
    s.over = true; s.won = true;
    s.flash = 0.5;
  }
  return true;
}

function undo(s) {
  const last = s.history.pop();
  if (!last) return false;
  s.block = last.block;
  s.moves = Math.max(0, s.moves - 1);
  s.over = false; s.won = false; s.fell = false;
  return true;
}

function restart(s) {
  const p = parseLevel(s.lv);
  s.block = { col: p.startCol, row: p.startRow, orient: 'up' };
  s.moves = 0;
  s.history.length = 0;
  s.over = false; s.won = false; s.fell = false;
}

// 3 stars at or under par; 2 within +50%; 1 otherwise.
function stars(moves, par) {
  if (moves <= par) return 3;
  if (moves <= par + Math.ceil(par * 0.5)) return 2;
  return 1;
}
