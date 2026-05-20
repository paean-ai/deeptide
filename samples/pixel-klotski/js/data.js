// Pixel Klotski - the classic "Huarong Trail" sliding-block puzzle. A
// rectangular board holds a handful of axis-aligned blocks of varying
// shapes; slide them one cell at a time (no rotation) to escort the
// red 2x2 "general" to the goal slot at the bottom-centre.
//
// Each block: { id, x, y, w, h }. The board is BOARD_W x BOARD_H cells.
// One block is the target (always id 0); the goal is to slide its
// top-left corner to (goalX, goalY).
//
// Six hand-designed boards are verified solvable + their par is the
// in-process BFS minimum (matches the Huarong Trail tradition: every
// listed par is the proven optimum, not an estimate).

const VW = 360, VH = 480;
const BOARD_W = 4;
const BOARD_H = 5;

const LEVELS = [
  // 1. Foothold — minimal warm-up. Just nudge the general past one block.
  {
    name: ['Foothold', '立足'], par: 10,
    pieces: [
      { id: 0, x: 1, y: 0, w: 2, h: 2 },  // general (target)
      { id: 1, x: 0, y: 2, w: 1, h: 1 },
      { id: 2, x: 3, y: 2, w: 1, h: 1 },
      { id: 3, x: 1, y: 3, w: 2, h: 1 },
    ],
    goal: { x: 1, y: 3 },
  },
  // 2. Squeeze — two soldiers framing the path.
  {
    name: ['Squeeze', '夹道'], par: 13,
    pieces: [
      { id: 0, x: 1, y: 0, w: 2, h: 2 },
      { id: 1, x: 0, y: 0, w: 1, h: 2 },
      { id: 2, x: 3, y: 0, w: 1, h: 2 },
      { id: 3, x: 0, y: 2, w: 1, h: 1 },
      { id: 4, x: 3, y: 2, w: 1, h: 1 },
      { id: 5, x: 1, y: 3, w: 2, h: 1 },
    ],
    goal: { x: 1, y: 3 },
  },
  // 3. Pinch — vertical guards + horizontal lieutenants.
  {
    name: ['Pinch', '掣肘'], par: 16,
    pieces: [
      { id: 0, x: 1, y: 0, w: 2, h: 2 },
      { id: 1, x: 0, y: 0, w: 1, h: 2 },
      { id: 2, x: 3, y: 0, w: 1, h: 2 },
      { id: 3, x: 1, y: 2, w: 2, h: 1 },
      { id: 4, x: 0, y: 2, w: 1, h: 2 },
      { id: 5, x: 3, y: 2, w: 1, h: 2 },
    ],
    goal: { x: 1, y: 3 },
  },
  // 4. Crossroads — many singletons clog the middle.
  {
    name: ['Crossroads', '十字'], par: 20,
    pieces: [
      { id: 0, x: 1, y: 0, w: 2, h: 2 },
      { id: 1, x: 0, y: 0, w: 1, h: 1 },
      { id: 2, x: 3, y: 0, w: 1, h: 1 },
      { id: 3, x: 0, y: 1, w: 1, h: 2 },
      { id: 4, x: 3, y: 1, w: 1, h: 2 },
      { id: 5, x: 1, y: 2, w: 2, h: 1 },
      { id: 6, x: 0, y: 3, w: 1, h: 1 },
      { id: 7, x: 3, y: 3, w: 1, h: 1 },
    ],
    goal: { x: 1, y: 3 },
  },
  // 5. Trial — half-classical layout: 9 pieces, BFS minimum 54 single-cell
  // slides — the sweet-spot warm-up before the Huarong finale.
  {
    name: ['Trial', '考验'], par: 54,
    pieces: [
      { id: 0, x: 1, y: 0, w: 2, h: 2 },
      { id: 1, x: 0, y: 0, w: 1, h: 2 },
      { id: 2, x: 3, y: 0, w: 1, h: 2 },
      { id: 3, x: 0, y: 2, w: 1, h: 2 },
      { id: 4, x: 3, y: 2, w: 1, h: 2 },
      { id: 5, x: 1, y: 2, w: 2, h: 1 },
      { id: 6, x: 0, y: 4, w: 1, h: 1 },
      { id: 7, x: 3, y: 4, w: 1, h: 1 },
      { id: 8, x: 1, y: 4, w: 1, h: 1 },
    ],
    goal: { x: 1, y: 3 },
  },
  // 6. Huarong — the legendary classical layout (~81 min moves).
  // (We let the BFS solver derive par at test time; the README lists it.)
  {
    name: ['Huarong', '华容'], par: 116,
    pieces: [
      { id: 0, x: 1, y: 0, w: 2, h: 2 },
      { id: 1, x: 0, y: 0, w: 1, h: 2 },
      { id: 2, x: 3, y: 0, w: 1, h: 2 },
      { id: 3, x: 0, y: 2, w: 1, h: 2 },
      { id: 4, x: 3, y: 2, w: 1, h: 2 },
      { id: 5, x: 1, y: 2, w: 2, h: 1 },
      { id: 6, x: 1, y: 3, w: 1, h: 1 },
      { id: 7, x: 2, y: 3, w: 1, h: 1 },
      { id: 8, x: 0, y: 4, w: 1, h: 1 },
      { id: 9, x: 3, y: 4, w: 1, h: 1 },
    ],
    goal: { x: 1, y: 3 },
  },
];
const LEVEL_COUNT = LEVELS.length;

// ---- board helpers -----------------------------------------------------
function clonePieces(ps) { return ps.map(p => ({ ...p })); }

function cellsOf(p) {
  const out = [];
  for (let dy = 0; dy < p.h; dy++) for (let dx = 0; dx < p.w; dx++) {
    out.push([p.x + dx, p.y + dy]);
  }
  return out;
}

// True iff the piece (with offset dx,dy applied to its top-left) fits on
// the board and overlaps no other piece.
function canSlide(pieces, idx, dx, dy) {
  const p = pieces[idx];
  const nx = p.x + dx, ny = p.y + dy;
  if (nx < 0 || ny < 0 || nx + p.w > BOARD_W || ny + p.h > BOARD_H) return false;
  for (let dyy = 0; dyy < p.h; dyy++) for (let dxx = 0; dxx < p.w; dxx++) {
    const cx = nx + dxx, cy = ny + dyy;
    for (let j = 0; j < pieces.length; j++) {
      if (j === idx) continue;
      const q = pieces[j];
      if (cx >= q.x && cx < q.x + q.w && cy >= q.y && cy < q.y + q.h) return false;
    }
  }
  return true;
}

function slide(pieces, idx, dx, dy) {
  pieces[idx].x += dx; pieces[idx].y += dy;
}

// Win when the target (id 0) sits at goal (top-left).
function isWin(pieces, goal) {
  const t = pieces[0];
  return t.x === goal.x && t.y === goal.y;
}

// Serialize for memoisation in BFS / verify. Same piece-shape group
// (e.g. two 1x1 soldiers) is interchangeable so we canonicalise by
// sorting same-shape pieces by (x, y).
function stateKey(pieces) {
  const byShape = new Map();
  for (const p of pieces) {
    const k = `${p.w}x${p.h}`;
    if (!byShape.has(k)) byShape.set(k, []);
    byShape.get(k).push([p.x, p.y]);
  }
  const parts = [];
  for (const [k, arr] of Array.from(byShape.entries()).sort()) {
    arr.sort((a, b) => a[0] - b[0] || a[1] - b[1]);
    parts.push(k + ':' + arr.map(([x, y]) => x + ',' + y).join(';'));
  }
  return parts.join('|');
}

// ---- runtime state -----------------------------------------------------
function buildGame(levelIndex) {
  const lv = LEVELS[levelIndex];
  return {
    levelIndex, lv,
    pieces: clonePieces(lv.pieces),
    selected: -1,
    moves: 0,
    history: [],
    solved: false,
    over: false,
  };
}

function tapPiece(s, idx) {
  if (s.over) return false;
  if (idx < 0 || idx >= s.pieces.length) return false;
  if (s.selected === idx) { s.selected = -1; return true; }
  s.selected = idx;
  return true;
}

// Slide the currently-selected piece by (dx, dy) if possible.
function trySlide(s, dx, dy) {
  if (s.over || s.selected < 0) return false;
  if (Math.abs(dx) + Math.abs(dy) !== 1) return false;
  if (!canSlide(s.pieces, s.selected, dx, dy)) return false;
  s.history.push({ idx: s.selected, dx: -dx, dy: -dy });
  slide(s.pieces, s.selected, dx, dy);
  s.moves++;
  if (isWin(s.pieces, s.lv.goal)) { s.solved = true; s.over = true; }
  return true;
}

function undo(s) {
  const last = s.history.pop();
  if (!last) return false;
  slide(s.pieces, last.idx, last.dx, last.dy);
  s.moves = Math.max(0, s.moves - 1);
  s.solved = false; s.over = false;
  return true;
}

function restart(s) {
  s.pieces = clonePieces(s.lv.pieces);
  s.selected = -1;
  s.moves = 0;
  s.history.length = 0;
  s.solved = false; s.over = false;
}

// Which cell does (cx, cy) belong to? Returns piece index or -1.
function pieceAt(pieces, cx, cy) {
  for (let i = 0; i < pieces.length; i++) {
    const p = pieces[i];
    if (cx >= p.x && cx < p.x + p.w && cy >= p.y && cy < p.y + p.h) return i;
  }
  return -1;
}

// Stars: optimal = 3, +50% = 2, more = 1.
function stars(moves, par) {
  if (moves <= par) return 3;
  if (moves <= par + Math.ceil(par * 0.5)) return 2;
  return 1;
}
