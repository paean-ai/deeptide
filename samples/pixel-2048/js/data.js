// Pixel 2048 - slide tiles in a direction and merge any two adjacent
// equal-value tiles into one tile of double the value. Every move
// spawns a new 2 (90%) or 4 (10%) tile in an empty cell. Reach 2048
// to win — keep going for higher tiles, or stop when no slide is
// possible.

const VW = 360, VH = 480;
const SIZE = 4;
const WIN_VALUE = 2048;

// ---- core slide/merge --------------------------------------------------
// All four directions reduce to a "slide-left" on a row: drop zeros,
// merge consecutive equals (one merge per cell per move), pad with zeros.
function slideRowLeft(row) {
  const compact = row.filter(v => v !== 0);
  const out = [];
  let gained = 0;
  for (let i = 0; i < compact.length; i++) {
    if (i + 1 < compact.length && compact[i] === compact[i + 1]) {
      const v = compact[i] * 2;
      out.push(v);
      gained += v;
      i++;                          // skip the partner; one merge per slide
    } else {
      out.push(compact[i]);
    }
  }
  while (out.length < row.length) out.push(0);
  // Was the row actually changed? Used by the caller to gate spawn.
  let changed = false;
  for (let i = 0; i < row.length; i++) if (row[i] !== out[i]) { changed = true; break; }
  return { row: out, gained, changed };
}

function getRow(grid, y) { return grid[y].slice(); }
function setRow(grid, y, row) { for (let i = 0; i < SIZE; i++) grid[y][i] = row[i]; }
function getCol(grid, x) { const r = []; for (let y = 0; y < SIZE; y++) r.push(grid[y][x]); return r; }
function setCol(grid, x, col) { for (let y = 0; y < SIZE; y++) grid[y][x] = col[y]; }

function slide(grid, dir) {
  // dir: 0 = up, 1 = right, 2 = down, 3 = left
  let any = false, gained = 0;
  if (dir === 3) {
    for (let y = 0; y < SIZE; y++) {
      const r = slideRowLeft(getRow(grid, y));
      setRow(grid, y, r.row);
      gained += r.gained; if (r.changed) any = true;
    }
  } else if (dir === 1) {
    for (let y = 0; y < SIZE; y++) {
      const row = getRow(grid, y).reverse();
      const r = slideRowLeft(row);
      setRow(grid, y, r.row.reverse());
      gained += r.gained; if (r.changed) any = true;
    }
  } else if (dir === 0) {
    for (let x = 0; x < SIZE; x++) {
      const r = slideRowLeft(getCol(grid, x));
      setCol(grid, x, r.row);
      gained += r.gained; if (r.changed) any = true;
    }
  } else if (dir === 2) {
    for (let x = 0; x < SIZE; x++) {
      const col = getCol(grid, x).reverse();
      const r = slideRowLeft(col);
      setCol(grid, x, r.row.reverse());
      gained += r.gained; if (r.changed) any = true;
    }
  }
  return { changed: any, gained };
}

function emptyCells(grid) {
  const out = [];
  for (let y = 0; y < SIZE; y++) for (let x = 0; x < SIZE; x++)
    if (grid[y][x] === 0) out.push([x, y]);
  return out;
}

function spawnTile(grid) {
  const empties = emptyCells(grid);
  if (!empties.length) return null;
  const [x, y] = empties[Math.floor(Math.random() * empties.length)];
  const v = Math.random() < 0.1 ? 4 : 2;
  grid[y][x] = v;
  return { x, y, v };
}

function anyMove(grid) {
  for (let y = 0; y < SIZE; y++) for (let x = 0; x < SIZE; x++) {
    if (grid[y][x] === 0) return true;
    if (x + 1 < SIZE && grid[y][x] === grid[y][x + 1]) return true;
    if (y + 1 < SIZE && grid[y][x] === grid[y + 1][x]) return true;
  }
  return false;
}

function maxTile(grid) {
  let m = 0;
  for (const r of grid) for (const v of r) if (v > m) m = v;
  return m;
}

// ---- game state --------------------------------------------------------
function newGame() {
  const grid = Array.from({ length: SIZE }, () => Array(SIZE).fill(0));
  spawnTile(grid); spawnTile(grid);
  return {
    grid,
    score: 0,
    moves: 0,
    won: false,           // crossed 2048 at least once this run
    keepPlaying: false,   // user dismissed the win modal and is going on
    over: false,
    history: null,        // snapshot for single-step undo
    lastSpawn: null,
  };
}

function snapshot(s) {
  return {
    grid: s.grid.map(r => r.slice()),
    score: s.score,
    moves: s.moves,
    won: s.won,
    keepPlaying: s.keepPlaying,
    over: s.over,
  };
}
function restore(s, snap) {
  s.grid = snap.grid.map(r => r.slice());
  s.score = snap.score;
  s.moves = snap.moves;
  s.won = snap.won;
  s.keepPlaying = snap.keepPlaying;
  s.over = snap.over;
}

function move(s, dir) {
  if (s.over) return false;
  const before = snapshot(s);
  const { changed, gained } = slide(s.grid, dir);
  if (!changed) return false;
  s.history = before;
  s.score += gained;
  s.moves++;
  s.lastSpawn = spawnTile(s.grid);
  if (!s.won && maxTile(s.grid) >= WIN_VALUE) s.won = true;
  if (!anyMove(s.grid)) s.over = true;
  return true;
}

function undo(s) {
  if (!s.history) return false;
  restore(s, s.history);
  s.history = null;
  s.lastSpawn = null;
  return true;
}
