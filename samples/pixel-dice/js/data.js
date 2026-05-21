// Pixel Dice - a die-rolling logic puzzle.
//
// A six-sided die sits on a grid. Roll it one cell at a time - rolling
// tips its faces over. Some cells are seals stamped with a number; roll
// the die onto a seal while that number is face-UP and the seal is set.
// Set every seal to clear the board.
//
// Each level is { n, k, minPar, seed }. buildPuzzle generates the board by
// construction (a random walk that stamps all seals exists), so a solution
// is guaranteed; the listed par is the BFS minimum.

const VW = 360, VH = 480;

// A standard die: opposite faces sum to 7.
const START_ORI = { t: 1, b: 6, n: 2, s: 5, e: 3, w: 4 };
// Roll directions: [name, dx, dy].
const DIRS = [['E', 1, 0], ['W', -1, 0], ['S', 0, 1], ['N', 0, -1]];

const LEVELS = [
  { name: ['Pip',     '点'],   n: 5, k: 3, minPar: 7,  seed: 311 },
  { name: ['Tumble',  '翻滚'], n: 5, k: 4, minPar: 9,  seed: 437 },
  { name: ['Cascade', '连转'], n: 6, k: 4, minPar: 10, seed: 553 },
  { name: ['Lattice', '格阵'], n: 6, k: 5, minPar: 12, seed: 671 },
  { name: ['Gauntlet','试炼'], n: 7, k: 5, minPar: 14, seed: 793 },
  { name: ['Cyclone', '回旋'], n: 7, k: 6, minPar: 16, seed: 911 },
];
const LEVEL_COUNT = LEVELS.length;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () { s = (s * 16807) % 2147483647; return (s - 1) / 2147483646; };
}

// Roll the die one cell; faces tip over.
function roll(o, dir) {
  if (dir === 'E') return { t: o.w, e: o.t, b: o.e, w: o.b, n: o.n, s: o.s };
  if (dir === 'W') return { t: o.e, e: o.b, b: o.w, w: o.t, n: o.n, s: o.s };
  if (dir === 'S') return { t: o.n, n: o.b, b: o.s, s: o.t, e: o.e, w: o.w };
  return            { t: o.s, s: o.b, b: o.n, n: o.t, e: o.e, w: o.w };   // 'N'
}
function oriKey(o) { return o.t * 36 + o.n * 6 + o.e; }

// ---- BFS shortest solution ---------------------------------------------
function solvePar(n, start, goals) {
  const goalCells = Object.keys(goals).map(Number);
  const ALL = (1 << goalCells.length) - 1;
  const key = (c, o, m) => c + ',' + oriKey(o) + ',' + m;
  const q = [{ c: start, o: START_ORI, m: 0, d: 0 }];
  const seen = new Set([key(start, START_ORI, 0)]);
  let head = 0;
  while (head < q.length) {
    const cur = q[head++];
    if (cur.m === ALL) return cur.d;
    const x = cur.c % n, y = (cur.c / n) | 0;
    for (const [dir, dx, dy] of DIRS) {
      const nx = x + dx, ny = y + dy;
      if (nx < 0 || nx >= n || ny < 0 || ny >= n) continue;
      const nc = ny * n + nx;
      const no = roll(cur.o, dir);
      let nm = cur.m;
      const gi = goalCells.indexOf(nc);
      if (gi >= 0 && goals[nc] === no.t) nm |= (1 << gi);
      const k = key(nc, no, nm);
      if (seen.has(k)) continue;
      seen.add(k);
      q.push({ c: nc, o: no, m: nm, d: cur.d + 1 });
    }
  }
  return -1;
}

// ---- generation --------------------------------------------------------
// Walk the die at random, then make seals out of cells it visited (with a
// face it actually showed there) - so the walk itself is a valid solution.
function buildPuzzle(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const n = cfg.n;
  for (let attempt = 0; attempt < 400; attempt++) {
    const rng = seededRandom(cfg.seed + attempt * 1009);
    const start = ((rng() * n) | 0) + ((rng() * n) | 0) * n;
    let c = start, o = START_ORI;
    const byCell = {};
    for (let step = 0; step < n * n * 3; step++) {
      const x = c % n, y = (c / n) | 0;
      const opts = DIRS.filter(([d, dx, dy]) => {
        const nx = x + dx, ny = y + dy;
        return nx >= 0 && nx < n && ny >= 0 && ny < n;
      });
      const [dir, dx, dy] = opts[(rng() * opts.length) | 0];
      c = (y + dy) * n + (x + dx);
      o = roll(o, dir);
      if (c !== start) {
        if (!byCell[c]) byCell[c] = [];
        byCell[c].push(o.t);
      }
    }
    const cells = Object.keys(byCell).map(Number);
    for (let i = cells.length - 1; i > 0; i--) {
      const j = (rng() * (i + 1)) | 0;
      const tmp = cells[i]; cells[i] = cells[j]; cells[j] = tmp;
    }
    if (cells.length < cfg.k) continue;
    const goals = {};
    for (let i = 0; i < cfg.k; i++) {
      const cc = cells[i], tops = byCell[cc];
      goals[cc] = tops[(rng() * tops.length) | 0];
    }
    const par = solvePar(n, start, goals);
    if (par >= cfg.minPar) {
      return { n, start, goals, par, levelIndex, cfg };
    }
  }
  return null;
}

// ---- play state --------------------------------------------------------
function newPlay(puzzle) {
  const s = {
    puzzle, cell: puzzle.start, ori: Object.assign({}, START_ORI),
    stamped: {}, moves: 0, history: [], over: false,
  };
  return s;
}

function adjacentRoll(s, dir) {
  const n = s.puzzle.n, x = s.cell % n, y = (s.cell / n) | 0;
  const def = DIRS.find(d => d[0] === dir);
  const nx = x + def[1], ny = y + def[2];
  if (nx < 0 || nx >= n || ny < 0 || ny >= n) return null;
  return { cell: ny * n + nx, ori: roll(s.ori, dir) };
}

function rollDie(s, dir) {
  if (s.over) return false;
  const r = adjacentRoll(s, dir);
  if (!r) return false;
  s.history.push({ cell: s.cell, ori: s.ori, stamped: Object.assign({}, s.stamped) });
  s.cell = r.cell;
  s.ori = r.ori;
  s.moves++;
  // Stamp a seal if the die landed on it face-up.
  if (s.puzzle.goals[s.cell] !== undefined && s.puzzle.goals[s.cell] === s.ori.t) {
    s.stamped[s.cell] = true;
  }
  if (isSolved(s)) s.over = true;
  return true;
}

function undo(s) {
  const h = s.history.pop();
  if (!h) return false;
  s.cell = h.cell; s.ori = h.ori; s.stamped = h.stamped;
  s.moves = Math.max(0, s.moves - 1);
  s.over = false;
  return true;
}

function restart(s) {
  s.cell = s.puzzle.start;
  s.ori = Object.assign({}, START_ORI);
  s.stamped = {};
  s.moves = 0;
  s.history.length = 0;
  s.over = false;
}

function isSolved(s) {
  for (const c in s.puzzle.goals) if (!s.stamped[c]) return false;
  return true;
}

// 3 stars at or under par, 2 within +50%, else 1.
function stars(moves, par) {
  if (moves <= par) return 3;
  if (moves <= par + Math.ceil(par * 0.5)) return 2;
  return 1;
}
