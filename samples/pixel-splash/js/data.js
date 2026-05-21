// Pixel Splash - flood-fill paint puzzle. Pure logic: generation + solver.
//
// Paint floods out from the top-left cell. Each move recolours the whole
// connected splash to a chosen paint; it then merges with any newly-touching
// patch of that colour. Cover every paintable cell - stones never take paint
// and just shape the route - inside the move budget. The budget is the greedy
// reference solver's move count plus three, so every level is winnable.

const VW = 360, VH = 480;
const STONE = -1;

const LEVELS = [
  { name: ['Studio',     '画室'],   seed: 17,  n: 6,  colors: 4, stones: 0 },
  { name: ['Atelier',    '工坊'],   seed: 53,  n: 7,  colors: 4, stones: 2 },
  { name: ['Gallery',    '画廊'],   seed: 131, n: 8,  colors: 5, stones: 4 },
  { name: ['Mural',      '壁画'],   seed: 247, n: 9,  colors: 5, stones: 5 },
  { name: ['Fresco',     '湿壁画'], seed: 389, n: 10, colors: 6, stones: 7 },
  { name: ['Masterwork', '杰作'],   seed: 547, n: 11, colors: 6, stones: 9 },
];
const LEVEL_COUNT = LEVELS.length;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () {
    s = (s * 16807) % 2147483647;
    return (s - 1) / 2147483646;
  };
}

const NB = [[1, 0], [-1, 0], [0, 1], [0, -1]];

// The splash region: same-colour cells 4-connected to the origin, stones
// excluded. Returns a Uint8Array mask over the board.
function originRegion(board, n) {
  const mask = new Uint8Array(n * n);
  const c = board[0];
  if (c === STONE) return mask;
  const stack = [0]; mask[0] = 1;
  while (stack.length) {
    const cell = stack.pop();
    const r = (cell / n) | 0, col = cell % n;
    for (const [dr, dc] of NB) {
      const nr = r + dr, nc = col + dc;
      if (nr < 0 || nc < 0 || nr >= n || nc >= n) continue;
      const ni = nr * n + nc;
      if (!mask[ni] && board[ni] === c) { mask[ni] = 1; stack.push(ni); }
    }
  }
  return mask;
}

function nonStoneCount(board) {
  let k = 0;
  for (let i = 0; i < board.length; i++) if (board[i] !== STONE) k++;
  return k;
}

function regionSize(board, n) {
  const m = originRegion(board, n);
  let k = 0;
  for (let i = 0; i < m.length; i++) k += m[i];
  return k;
}

function isWon(board, n) {
  return regionSize(board, n) === nonStoneCount(board);
}

// Recolour the splash region to colour c, in place.
function applyColor(board, n, c) {
  const m = originRegion(board, n);
  for (let i = 0; i < m.length; i++) if (m[i]) board[i] = c;
}

// Greedy reference solver: each move picks whichever colour absorbs the most
// new cells. Every move strictly grows the splash, so it always finishes.
function greedySolve(board, n, colors) {
  const work = board.slice();
  const moves = [];
  let guard = 0;
  while (!isWon(work, n) && guard < n * n * 4) {
    guard++;
    const cur = work[0];
    const before = regionSize(work, n);
    let best = -1, bestGain = -1;
    for (let c = 0; c < colors; c++) {
      if (c === cur) continue;
      const t = work.slice();
      applyColor(t, n, c);
      const gain = regionSize(t, n) - before;
      if (gain > bestGain) { bestGain = gain; best = c; }
    }
    if (best < 0) break;
    applyColor(work, n, best);
    moves.push(best);
  }
  return moves;
}

// Are all non-stone cells reachable from the origin ignoring colour? Used so
// stone placement never seals off a patch the splash could not otherwise win.
function nonStoneConnected(board, n) {
  const seen = new Uint8Array(n * n);
  const stack = [0]; seen[0] = 1;
  while (stack.length) {
    const cell = stack.pop();
    const r = (cell / n) | 0, col = cell % n;
    for (const [dr, dc] of NB) {
      const nr = r + dr, nc = col + dc;
      if (nr < 0 || nc < 0 || nr >= n || nc >= n) continue;
      const ni = nr * n + nc;
      if (!seen[ni] && board[ni] !== STONE) { seen[ni] = 1; stack.push(ni); }
    }
  }
  for (let i = 0; i < board.length; i++) {
    if (board[i] !== STONE && !seen[i]) return false;
  }
  return true;
}

function buildLevel(index) {
  const cfg = LEVELS[index];
  const n = cfg.n;
  const rng = seededRandom(cfg.seed);
  const board = new Array(n * n);
  for (let i = 0; i < n * n; i++) board[i] = (rng() * cfg.colors) | 0;
  // place stones one at a time, keeping the paintable area fully connected
  let placed = 0, attempts = 0;
  while (placed < cfg.stones && attempts < 3000) {
    attempts++;
    const i = (rng() * n * n) | 0;
    if (i === 0 || board[i] === STONE) continue;
    const prev = board[i];
    board[i] = STONE;
    if (nonStoneConnected(board, n)) placed++;
    else board[i] = prev;
  }
  const par = greedySolve(board, n, cfg.colors).length;
  return { index, name: cfg.name, n, colors: cfg.colors, board, par, budget: par + 3 };
}

// ---- play state ----------------------------------------------------------
function newPlay(level) {
  return { level, board: level.board.slice(), moves: 0, history: [], over: false, won: false };
}

function pickColor(s, c) {
  if (s.over) return false;
  if (c < 0 || c >= s.level.colors || c === s.board[0]) return false;
  s.history.push({ board: s.board.slice(), moves: s.moves });
  applyColor(s.board, s.level.n, c);
  s.moves++;
  if (isWon(s.board, s.level.n)) { s.over = true; s.won = true; }
  else if (s.moves >= s.level.budget) { s.over = true; s.won = false; }
  return true;
}

function undo(s) {
  if (!s.history.length) return false;
  const h = s.history.pop();
  s.board = h.board;
  s.moves = h.moves;
  s.over = false; s.won = false;
  return true;
}

function restart(s) {
  s.board = s.level.board.slice();
  s.moves = 0; s.history = []; s.over = false; s.won = false;
}

function stars(moves, par) {
  if (moves <= par) return 3;
  if (moves <= par + 1) return 2;
  return 1;
}
