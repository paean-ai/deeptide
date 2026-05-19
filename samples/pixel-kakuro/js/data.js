// Pixel Kakuro - puzzle campaign, clue derivation and the verifying solver.
//
// Each puzzle is a wall/white SHAPE plus a seed. From the seed a digit fill is
// generated (no repeated digit in any run) and accepted only when the run sums
// it produces have exactly ONE consistent solution - so every level is a
// genuine, uniquely-solvable kakuro.

const VW = 360, VH = 480;

const PUZZLES = [
  { name: ['Starter', '入门'], seed: 7, shape: [
    '#####',
    '#..##',
    '#...#',
    '##..#',
  ] },
  { name: ['Corner', '一角'], seed: 31, shape: [
    '######',
    '#..###',
    '#...##',
    '##...#',
    '###..#',
  ] },
  { name: ['Stairs', '阶梯'], seed: 52, shape: [
    '######',
    '#..###',
    '#...##',
    '##...#',
    '###..#',
    '###..#',
  ] },
  { name: ['Window', '窗格'], seed: 88, shape: [
    '######',
    '#..#..',
    '#.....',
    '##...#',
    '##..##',
  ] },
  { name: ['Helix', '螺旋'], seed: 140, shape: [
    '######',
    '###..#',
    '##...#',
    '#...##',
    '#..###',
    '#..###',
  ] },
  { name: ['Cascade', '瀑布'], seed: 203, shape: [
    '#######',
    '#..####',
    '#....##',
    '##....#',
    '####..#',
  ] },
];
const PUZZLE_COUNT = PUZZLES.length;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () {
    s = (s * 16807) % 2147483647;
    return (s - 1) / 2147483646;
  };
}

// Parse a wall/white shape into geometry.
function parseShape(shape) {
  const h = shape.length, w = shape[0].length;
  const white = [];
  for (let r = 0; r < h; r++) {
    white.push([]);
    for (let c = 0; c < w; c++) white[r].push(shape[r][c] !== '#');
  }
  return { w, h, white };
}

// All horizontal + vertical runs of length >= 2: { cells, dir }.
function runsOf(g) {
  const runs = [];
  for (let r = 0; r < g.h; r++) {
    let cur = [];
    for (let c = 0; c <= g.w; c++) {
      if (c < g.w && g.white[r][c]) cur.push([r, c]);
      else { if (cur.length >= 2) runs.push({ cells: cur, dir: 'h' }); cur = []; }
    }
  }
  for (let c = 0; c < g.w; c++) {
    let cur = [];
    for (let r = 0; r <= g.h; r++) {
      if (r < g.h && g.white[r][c]) cur.push([r, c]);
      else { if (cur.length >= 2) runs.push({ cells: cur, dir: 'v' }); cur = []; }
    }
  }
  return runs;
}

// Clue cells: a wall holding the sum of the run starting just right / below.
function cluesOf(g, runs) {
  const map = {};
  for (const run of runs) {
    const [r0, c0] = run.cells[0];
    const k = run.dir === 'h' ? r0 + ',' + (c0 - 1) : (r0 - 1) + ',' + c0;
    (map[k] || (map[k] = {}))[run.dir === 'h' ? 'right' : 'down'] = run.sum;
  }
  return map;
}

// Count solutions consistent with the run sums, stopping at `limit`.
function solveCount(g, runs, limit) {
  const cells = [];
  for (let r = 0; r < g.h; r++) for (let c = 0; c < g.w; c++) if (g.white[r][c]) cells.push([r, c]);
  const cellRuns = {};
  for (const [r, c] of cells) cellRuns[r + ',' + c] = [];
  for (const run of runs) for (const [r, c] of run.cells) cellRuns[r + ',' + c].push(run);
  const val = {};
  let found = 0;

  function runOk(run, full) {
    const seen = new Set();
    let sum = 0, filled = 0;
    for (const [r, c] of run.cells) {
      const v = val[r + ',' + c];
      if (v) { if (seen.has(v)) return false; seen.add(v); sum += v; filled++; }
    }
    const rem = run.cells.length - filled;
    if (full || rem === 0) return sum === run.sum;
    const avail = [];
    for (let d = 1; d <= 9; d++) if (!seen.has(d)) avail.push(d);
    let lo = 0, hi = 0;
    for (let i = 0; i < rem; i++) { lo += avail[i]; hi += avail[avail.length - 1 - i]; }
    return sum + lo <= run.sum && run.sum <= sum + hi;
  }
  function bt(i) {
    if (found >= limit) return;
    if (i === cells.length) {
      if (runs.every(run => runOk(run, true))) found++;
      return;
    }
    const [r, c] = cells[i];
    const my = cellRuns[r + ',' + c];
    for (let d = 1; d <= 9; d++) {
      val[r + ',' + c] = d;
      if (my.every(run => runOk(run, false))) bt(i + 1);
      if (found >= limit) { delete val[r + ',' + c]; return; }
    }
    delete val[r + ',' + c];
  }
  bt(0);
  return found;
}

// Generate a uniquely-solvable kakuro for a shape from its seed.
function buildPuzzle(p) {
  const g = parseShape(p.shape);
  const runs = runsOf(g);
  const cells = [];
  for (let r = 0; r < g.h; r++) for (let c = 0; c < g.w; c++) if (g.white[r][c]) cells.push([r, c]);
  const cellRuns = {};
  for (const [r, c] of cells) cellRuns[r + ',' + c] = [];
  for (const run of runs) for (const [r, c] of run.cells) cellRuns[r + ',' + c].push(run);
  const rng = seededRandom(p.seed);

  // a random digit fill with no repeat inside any run
  function randomFill() {
    const val = {};
    function ok(r, c, d) {
      for (const run of cellRuns[r + ',' + c]) {
        for (const [rr, cc] of run.cells) if (val[rr + ',' + cc] === d) return false;
      }
      return true;
    }
    function bt(i) {
      if (i === cells.length) return true;
      const [r, c] = cells[i];
      const order = [1, 2, 3, 4, 5, 6, 7, 8, 9];
      for (let k = order.length - 1; k > 0; k--) {
        const j = (rng() * (k + 1)) | 0;
        [order[k], order[j]] = [order[j], order[k]];
      }
      for (const d of order) {
        if (!ok(r, c, d)) continue;
        val[r + ',' + c] = d;
        if (bt(i + 1)) return true;
        delete val[r + ',' + c];
      }
      return false;
    }
    return bt(0) ? val : null;
  }

  for (let attempt = 0; attempt < 6000; attempt++) {
    const val = randomFill();
    if (!val) continue;
    for (const run of runs) {
      run.sum = 0;
      for (const [r, c] of run.cells) run.sum += val[r + ',' + c];
    }
    if (solveCount(g, runs, 2) === 1) {
      const ans = [];
      for (let r = 0; r < g.h; r++) {
        ans.push([]);
        for (let c = 0; c < g.w; c++) ans[r].push(g.white[r][c] ? val[r + ',' + c] : 0);
      }
      return { w: g.w, h: g.h, white: g.white, ans, runs, clues: cluesOf(g, runs) };
    }
  }
  return null;
}
