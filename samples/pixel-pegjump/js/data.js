// Pixel Peg Jump - classic peg solitaire. A peg jumps orthogonally over
// an adjacent peg into an empty hole, removing the jumped peg. Clear the
// board down to a single peg to solve a level.

const VW = 360, VH = 480;

// Board cells: '.' = empty hole; 'O' = peg; ' ' = not part of the board.
// Each level lists rows top-to-bottom. The first row is row 0; the first
// character is col 0. Boards are at most 9 wide / 9 tall so cells stay
// comfortably tappable inside the 360x480 frame.

// Levels were hand-picked from a search over rectangular and cross-shaped
// boards by an in-process DFS solver; each layout below is verified to
// reduce to a single peg. (Many small boards — e.g. the 5x5 plus with a
// central hole — have a Conway-style parity obstruction and cannot be
// solved to 1; those are deliberately excluded.)
const LEVELS = [
  { // 1. Sprout — 3x4 box, hole near a corner. Quick win for newcomers.
    name: ['Sprout', '萌芽'],
    rows: [
      '.OOO',
      'OOOO',
      'OOOO'
    ]
  },
  { // 2. Garden — 4x4 box, hole near top-edge.
    name: ['Garden', '小园'],
    rows: [
      'O.OO',
      'OOOO',
      'OOOO',
      'OOOO'
    ]
  },
  { // 3. Bridge — 3x6 long bar, hole offset on the right.
    name: ['Bridge', '小桥'],
    rows: [
      'OOOOOO',
      'OOOOOO',
      'OOOO.O'
    ]
  },
  { // 4. Plaza — 5x4 box, hole near the middle.
    name: ['Plaza', '广场'],
    rows: [
      'O.OOO',
      'OOOOO',
      'OOOOO',
      'OOOOO'
    ]
  },
  { // 5. Tower — full 5x5 box, hole at (2,1).
    name: ['Tower', '塔楼'],
    rows: [
      'OOOOO',
      'OO.OO',
      'OOOOO',
      'OOOOO',
      'OOOOO'
    ]
  },
  { // 6. Cathedral — the classic English 33 board with central hole.
    name: ['Cathedral', '大殿'],
    rows: [
      '  OOO  ',
      '  OOO  ',
      'OOOOOOO',
      'OOO.OOO',
      'OOOOOOO',
      '  OOO  ',
      '  OOO  '
    ]
  }
];
const LEVEL_COUNT = LEVELS.length;

// ---- board model -------------------------------------------------------
// In-memory cells use the same characters: '.', 'O', ' '. Coordinates are
// (x, y) with x = column index, y = row index.

function buildGame(levelIndex) {
  const lv = LEVELS[levelIndex];
  const grid = lv.rows.map(r => r.split(''));
  const h = grid.length;
  const w = grid.reduce((m, r) => Math.max(m, r.length), 0);
  // Right-pad short rows so coordinates are consistent.
  for (const r of grid) while (r.length < w) r.push(' ');
  return {
    levelIndex, lv, w, h,
    grid,
    sel: null,           // currently selected peg [x,y] or null
    history: [],         // stack of {from, jumped, to} for undo
    over: false, won: false
  };
}

function pegCount(s) {
  let c = 0;
  for (const r of s.grid) for (const v of r) if (v === 'O') c++;
  return c;
}

// Legal moves from (x,y): orthogonal jumps of distance 2 over a peg into
// an empty hole. Returns an array of {to:[tx,ty], jumped:[mx,my]}.
function movesFrom(s, x, y) {
  if (s.grid[y][x] !== 'O') return [];
  const out = [];
  for (const [dx, dy] of [[2, 0], [-2, 0], [0, 2], [0, -2]]) {
    const mx = x + dx / 2, my = y + dy / 2;
    const tx = x + dx, ty = y + dy;
    if (ty < 0 || ty >= s.h || tx < 0 || tx >= s.w) continue;
    if (my < 0 || my >= s.h || mx < 0 || mx >= s.w) continue;
    if (s.grid[ty][tx] !== '.') continue;
    if (s.grid[my][mx] !== 'O') continue;
    out.push({ to: [tx, ty], jumped: [mx, my] });
  }
  return out;
}

// Any move on the entire board?
function anyMove(s) {
  for (let y = 0; y < s.h; y++) for (let x = 0; x < s.w; x++)
    if (s.grid[y][x] === 'O' && movesFrom(s, x, y).length) return true;
  return false;
}

// ---- input actions -----------------------------------------------------
// Tap a cell. Picks up a peg, plays a move, or unselects.
// Returns true if state changed.
function tapCell(s, x, y) {
  if (s.over) return false;
  if (x < 0 || y < 0 || x >= s.w || y >= s.h) return false;
  const cell = s.grid[y][x];
  // No selection yet -> pick up a peg with at least one move.
  if (!s.sel) {
    if (cell === 'O' && movesFrom(s, x, y).length) { s.sel = [x, y]; return true; }
    return false;
  }
  // Same cell -> deselect.
  if (s.sel[0] === x && s.sel[1] === y) { s.sel = null; return true; }
  // Different peg -> swap selection if it has moves.
  if (cell === 'O') {
    if (movesFrom(s, x, y).length) { s.sel = [x, y]; return true; }
    return false;
  }
  // Empty hole -> try to play the move from current selection.
  if (cell === '.') {
    const moves = movesFrom(s, s.sel[0], s.sel[1]);
    const m = moves.find(mv => mv.to[0] === x && mv.to[1] === y);
    if (!m) return false;
    const from = [s.sel[0], s.sel[1]];
    s.history.push({ from, jumped: m.jumped, to: m.to.slice() });
    s.grid[from[1]][from[0]] = '.';
    s.grid[m.jumped[1]][m.jumped[0]] = '.';
    s.grid[m.to[1]][m.to[0]] = 'O';
    s.sel = null;
    checkEnd(s);
    return true;
  }
  return false;
}

function undo(s) {
  if (s.over) return false;
  const last = s.history.pop();
  if (!last) return false;
  s.grid[last.from[1]][last.from[0]] = 'O';
  s.grid[last.jumped[1]][last.jumped[0]] = 'O';
  s.grid[last.to[1]][last.to[0]] = '.';
  s.sel = null;
  return true;
}

function restart(s) {
  const lv = LEVELS[s.levelIndex];
  s.grid = lv.rows.map(r => {
    const arr = r.split('');
    while (arr.length < s.w) arr.push(' ');
    return arr;
  });
  s.sel = null;
  s.history.length = 0;
  s.over = false;
  s.won = false;
}

function checkEnd(s) {
  const pegs = pegCount(s);
  if (pegs <= 1) { s.over = true; s.won = true; return; }
  if (!anyMove(s)) { s.over = true; s.won = false; }
}

// 3 stars for 1 peg left; 2 for 2; 1 for 3+.
function stars(pegsLeft) {
  if (pegsLeft <= 1) return 3;
  if (pegsLeft === 2) return 2;
  return 1;
}
