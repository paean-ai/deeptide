// Pixel Knight - the classic Knight's Tour. From a fixed starting
// square, hop in L-shapes (the eight knight moves) and visit every
// square on the board exactly once. The eight reachable cells light
// up as targets; a Warnsdorff hint dims everything except the
// candidate(s) with the fewest onward legal moves — the textbook
// heuristic that lets you find a tour even on the 8x8 board.

const VW = 360, VH = 480;

// 6 levels mixing board sizes. Each `start` was verified at design
// time by an in-process Warnsdorff search to admit at least one open
// knight's tour (see /tmp/test_kn_probe.js in the repo tests).
const LEVELS = [
  { name: ['Squire',   '随从'], n: 5, sx: 0, sy: 0 },   // 5x5 = 25 cells
  { name: ['Knight',   '骑士'], n: 5, sx: 2, sy: 2 },   // 5x5 from centre (open tour exists)
  { name: ['Lance',    '长矛'], n: 6, sx: 0, sy: 0 },   // 6x6 = 36
  { name: ['Castle',   '城堡'], n: 6, sx: 2, sy: 3 },
  { name: ['Crown',    '王冠'], n: 7, sx: 0, sy: 0 },   // 7x7 = 49
  { name: ['Throne',   '王座'], n: 8, sx: 0, sy: 0 },   // 8x8 = 64 (the legendary)
];
const LEVEL_COUNT = LEVELS.length;

// The eight knight offsets.
const KNIGHT_MOVES = [
  [ 1, -2], [ 2, -1], [ 2,  1], [ 1,  2],
  [-1,  2], [-2,  1], [-2, -1], [-1, -2],
];

function inBounds(n, x, y) { return x >= 0 && y >= 0 && x < n && y < n; }

// Return the set of legal knight moves from (x, y) on an n x n board
// that haven't been visited yet (visited[y][x] truthy means visited).
function legalMoves(n, visited, x, y) {
  const out = [];
  for (const [dx, dy] of KNIGHT_MOVES) {
    const nx = x + dx, ny = y + dy;
    if (!inBounds(n, nx, ny)) continue;
    if (visited[ny][nx]) continue;
    out.push([nx, ny]);
  }
  return out;
}

// Warnsdorff onward-move count from (x, y): how many legal next moves
// would exist after stepping there?
function onwardDegree(n, visited, x, y) {
  let c = 0;
  for (const [dx, dy] of KNIGHT_MOVES) {
    const nx = x + dx, ny = y + dy;
    if (!inBounds(n, nx, ny)) continue;
    if (visited[ny][nx]) continue;
    c++;
  }
  return c;
}

// The Warnsdorff hint candidate(s): legal moves with the minimum
// onwardDegree (ties returned as a set).
function warnsdorffHints(n, visited, x, y) {
  const moves = legalMoves(n, visited, x, y);
  if (!moves.length) return [];
  let best = Infinity;
  const scored = moves.map(([mx, my]) => {
    // Temporarily mark the target so degree counts only further-out cells.
    visited[my][mx] = true;
    const d = onwardDegree(n, visited, mx, my);
    visited[my][mx] = false;
    if (d < best) best = d;
    return { x: mx, y: my, d };
  });
  return scored.filter(s => s.d === best).map(s => [s.x, s.y]);
}

// ---- runtime state -----------------------------------------------------
function buildGame(levelIndex) {
  const lv = LEVELS[levelIndex];
  const visited = Array.from({ length: lv.n }, () => Array(lv.n).fill(0));
  visited[lv.sy][lv.sx] = 1;   // move number 1 at the start
  return {
    levelIndex, lv,
    n: lv.n,
    visited,                   // 0 if unvisited, otherwise the 1-based move number
    cx: lv.sx, cy: lv.sy,
    moves: 1,                  // count of cells visited (incl. start)
    history: [{ x: lv.sx, y: lv.sy }],
    over: false, won: false,
    flash: 0,
  };
}

// Try to move to (tx, ty). Returns true if accepted.
function tryMove(s, tx, ty) {
  if (s.over) return false;
  const dx = tx - s.cx, dy = ty - s.cy;
  let valid = false;
  for (const [mdx, mdy] of KNIGHT_MOVES) {
    if (mdx === dx && mdy === dy) { valid = true; break; }
  }
  if (!valid) return false;
  if (!inBounds(s.n, tx, ty)) return false;
  if (s.visited[ty][tx]) return false;
  s.cx = tx; s.cy = ty;
  s.moves++;
  s.visited[ty][tx] = s.moves;
  s.history.push({ x: tx, y: ty });
  if (s.moves === s.n * s.n) {
    s.over = true; s.won = true; s.flash = 0.55;
  } else if (legalMoves(s.n, s.visited, s.cx, s.cy).length === 0) {
    // Stuck — game ends as loss (cells remain unvisited).
    s.over = true; s.won = false; s.flash = 0.4;
  }
  return true;
}

function undo(s) {
  if (s.history.length <= 1) return false;
  const last = s.history.pop();
  s.visited[last.y][last.x] = 0;
  s.moves--;
  const prev = s.history[s.history.length - 1];
  s.cx = prev.x; s.cy = prev.y;
  s.over = false; s.won = false;
  return true;
}

function restart(s) {
  const lv = LEVELS[s.levelIndex];
  s.visited = Array.from({ length: lv.n }, () => Array(lv.n).fill(0));
  s.visited[lv.sy][lv.sx] = 1;
  s.cx = lv.sx; s.cy = lv.sy;
  s.moves = 1;
  s.history = [{ x: lv.sx, y: lv.sy }];
  s.over = false; s.won = false;
}

// Verify a tour exists from this start using Warnsdorff (used by tests).
function findTour(n, sx, sy) {
  const visited = Array.from({ length: n }, () => Array(n).fill(false));
  visited[sy][sx] = true;
  let cx = sx, cy = sy;
  for (let step = 1; step < n * n; step++) {
    const cands = warnsdorffHints(n, visited, cx, cy);
    if (!cands.length) return false;
    // Tie-break: first candidate (deterministic).
    const [nx, ny] = cands[0];
    visited[ny][nx] = true;
    cx = nx; cy = ny;
  }
  return true;
}

// Stars: 3 = complete tour, 2 = >= 80% visited, 1 = otherwise.
function stars(visitedCount, total) {
  if (visitedCount >= total) return 3;
  if (visitedCount >= total * 0.8) return 2;
  return 1;
}
