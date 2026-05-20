// Pixel Woody - drag-from-tray block puzzle. An 8x8 grid sits below a
// tray of three small wood blocks. Drag a block onto the grid (or tap
// the piece then tap a cell) and any row OR column that fills entirely
// clears, paying double-points. When all three tray pieces are placed,
// three fresh pieces appear. Game over when none of the three on offer
// can fit anywhere on the current board.

const VW = 360, VH = 480;

const GRID = 8;
const TRAY_SIZE = 3;

// Hand-curated bank of small polyominoes. Each is a list of [x, y]
// cells (normalised so the top-left of the bounding box is (0, 0)).
// Weights pick smaller pieces more often so the board stays playable.
const SHAPES = [
  { key: 'single',  cells: [[0,0]],                                   color: '#f4d27b', weight: 6 },
  { key: 'pair-h',  cells: [[0,0],[1,0]],                             color: '#5fc06e', weight: 4 },
  { key: 'pair-v',  cells: [[0,0],[0,1]],                             color: '#5fc06e', weight: 4 },
  { key: 'tri-h',   cells: [[0,0],[1,0],[2,0]],                       color: '#5fc0ff', weight: 3 },
  { key: 'tri-v',   cells: [[0,0],[0,1],[0,2]],                       color: '#5fc0ff', weight: 3 },
  { key: 'corner-tl', cells: [[0,0],[1,0],[0,1]],                     color: '#ff8fd0', weight: 3 },
  { key: 'corner-tr', cells: [[0,0],[1,0],[1,1]],                     color: '#ff8fd0', weight: 3 },
  { key: 'corner-bl', cells: [[0,0],[0,1],[1,1]],                     color: '#ff8fd0', weight: 3 },
  { key: 'corner-br', cells: [[1,0],[0,1],[1,1]],                     color: '#ff8fd0', weight: 3 },
  { key: 'square2',   cells: [[0,0],[1,0],[0,1],[1,1]],               color: '#bda6ff', weight: 3 },
  { key: 'i4-h',      cells: [[0,0],[1,0],[2,0],[3,0]],               color: '#e85a3a', weight: 2 },
  { key: 'i4-v',      cells: [[0,0],[0,1],[0,2],[0,3]],               color: '#e85a3a', weight: 2 },
  { key: 'l4',        cells: [[0,0],[0,1],[0,2],[1,2]],               color: '#a07a3a', weight: 2 },
  { key: 'j4',        cells: [[1,0],[1,1],[1,2],[0,2]],               color: '#a07a3a', weight: 2 },
  { key: 't4',        cells: [[0,0],[1,0],[2,0],[1,1]],               color: '#7a5fff', weight: 2 },
  { key: 'square3',   cells: [[0,0],[1,0],[2,0],[0,1],[1,1],[2,1],[0,2],[1,2],[2,2]], color: '#ffaa40', weight: 1 },
];

function totalWeight() {
  let n = 0; for (const s of SHAPES) n += s.weight;
  return n;
}
function pickShape(rng) {
  const total = totalWeight();
  let r = rng() * total;
  for (const s of SHAPES) {
    r -= s.weight;
    if (r <= 0) return s;
  }
  return SHAPES[0];
}

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () { s = (s * 16807) % 2147483647; return (s - 1) / 2147483646; };
}

// ---- board helpers -----------------------------------------------------
function emptyGrid() {
  const g = [];
  for (let y = 0; y < GRID; y++) {
    const row = [];
    for (let x = 0; x < GRID; x++) row.push(null);
    g.push(row);
  }
  return g;
}

function canPlace(grid, shape, ox, oy) {
  for (const [cx, cy] of shape.cells) {
    const x = ox + cx, y = oy + cy;
    if (x < 0 || y < 0 || x >= GRID || y >= GRID) return false;
    if (grid[y][x] !== null) return false;
  }
  return true;
}

function place(grid, shape, ox, oy) {
  for (const [cx, cy] of shape.cells) grid[oy + cy][ox + cx] = shape.color;
}

// Clear filled rows + cols; returns the count of lines cleared (rows + cols).
function clearLines(grid) {
  const fullRows = [];
  const fullCols = [];
  for (let y = 0; y < GRID; y++) {
    let full = true;
    for (let x = 0; x < GRID; x++) if (grid[y][x] === null) { full = false; break; }
    if (full) fullRows.push(y);
  }
  for (let x = 0; x < GRID; x++) {
    let full = true;
    for (let y = 0; y < GRID; y++) if (grid[y][x] === null) { full = false; break; }
    if (full) fullCols.push(x);
  }
  for (const y of fullRows) for (let x = 0; x < GRID; x++) grid[y][x] = null;
  for (const x of fullCols) for (let y = 0; y < GRID; y++) grid[y][x] = null;
  return { rows: fullRows.length, cols: fullCols.length, total: fullRows.length + fullCols.length };
}

// Does any (ox, oy) cell let this shape fit on the grid?
function shapeFitsAnywhere(grid, shape) {
  for (let y = 0; y <= GRID - 1; y++) for (let x = 0; x <= GRID - 1; x++) {
    if (canPlace(grid, shape, x, y)) return true;
  }
  return false;
}

// ---- runtime state -----------------------------------------------------
function newGame(seed) {
  const rng = seededRandom(seed || Math.floor(Date.now() % 2147483647));
  const grid = emptyGrid();
  const tray = [];
  for (let i = 0; i < TRAY_SIZE; i++) tray.push(pickShape(rng));
  return {
    rng, grid, tray,
    selected: -1,         // index into tray of the currently-picked piece
    score: 0,
    placed: 0,            // total pieces placed in the run
    over: false,
    flash: 0,
    lastClear: 0,         // count of lines cleared on the most recent placement
  };
}

// Refill the tray when it's emptied. If none of the 3 new pieces fit
// anywhere on the current grid, game over.
function refillIfNeeded(s) {
  if (s.tray.length > 0) return;
  for (let i = 0; i < TRAY_SIZE; i++) s.tray.push(pickShape(s.rng));
  // Check that at least ONE of the new pieces can fit somewhere.
  if (!s.tray.some(p => shapeFitsAnywhere(s.grid, p))) {
    s.over = true;
  }
}

// Try to place tray[idx] at grid (ox, oy). Returns true on success.
function tryPlace(s, idx, ox, oy) {
  if (s.over) return false;
  if (idx < 0 || idx >= s.tray.length) return false;
  const shape = s.tray[idx];
  if (!canPlace(s.grid, shape, ox, oy)) return false;
  place(s.grid, shape, ox, oy);
  s.placed++;
  s.score += shape.cells.length;
  s.tray.splice(idx, 1);
  s.selected = -1;
  // Line clears reward 10 * cells_cleared with a streak bonus per
  // additional line (so 1 line = 80, 2 lines = 80 + 100, etc.).
  const cleared = clearLines(s.grid);
  s.lastClear = cleared.total;
  if (cleared.total > 0) {
    s.score += cleared.total * 80 + (cleared.total - 1) * 20;
    s.flash = 0.4;
  }
  refillIfNeeded(s);
  // Mid-tray game over: if the player has chosen pieces and none of the
  // remaining can fit anywhere, end the run.
  if (s.tray.length > 0 && !s.tray.some(p => shapeFitsAnywhere(s.grid, p))) {
    s.over = true;
  }
  return true;
}

// Tap a tray piece to select / cancel.
function tapTray(s, idx) {
  if (s.over) return false;
  if (idx < 0 || idx >= s.tray.length) return false;
  if (s.selected === idx) { s.selected = -1; return true; }
  s.selected = idx;
  return true;
}

function tick(s, dt) {
  s.flash = Math.max(0, s.flash - dt);
}
