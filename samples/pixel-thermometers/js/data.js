// Pixel Thermometers - a logic puzzle.
//
// Rules:
//   * The grid is fully tiled by "thermometers" - each is a snake-shaped
//     run of cells with a round BULB at one end and a flat TIP at the
//     other.
//   * Mercury fills a thermometer CONTIGUOUSLY from the bulb: for some
//     amount k (0..length) the first k cells counting from the bulb are
//     mercury, the rest are empty. You can never have a gap.
//   * The number beside each row / column counts the mercury cells in it.
//   * Deduce the unique fill amount of every thermometer.
//
// Each level is { n, seed }. buildPuzzle is deterministic in the seed and
// returns { n, thermos, fills, rc, cc, owner, pos, levelIndex, cfg }.

const VW = 360, VH = 480;

const LEVELS = [
  { name: ['Chill',   '微凉'], n: 5, seed: 107 },
  { name: ['Tepid',   '温吞'], n: 5, seed: 251 },
  { name: ['Warm',    '暖意'], n: 6, seed: 329 },
  { name: ['Fever',   '发热'], n: 6, seed: 443 },
  { name: ['Searing', '灼热'], n: 7, seed: 557 },
  { name: ['Boiling', '沸腾'], n: 7, seed: 673 },
  { name: ['Scorching', '炙烤'], n: 7, seed: 801 },
  { name: ['Blazing',   '烈焰'], n: 7, seed: 911 },
  { name: ['Inferno',   '熔炉'], n: 8, seed: 1009 },
];
const LEVEL_COUNT = LEVELS.length;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () { s = (s * 16807) % 2147483647; return (s - 1) / 2147483646; };
}

// ---- thermometer layout ------------------------------------------------
// Tile the n x n grid with simple paths (each cell links orthogonally to
// the next; no branches). Every path has length >= 2; path[0] is the bulb.
function pathCover(n, rng, maxLen) {
  const total = n * n;
  for (let attempt = 0; attempt < 400; attempt++) {
    const used = new Array(total).fill(false);
    const paths = [];
    const order = [];
    for (let i = 0; i < total; i++) order.push(i);
    for (let i = total - 1; i > 0; i--) {
      const j = (rng() * (i + 1)) | 0;
      const tmp = order[i]; order[i] = order[j]; order[j] = tmp;
    }
    let ok = true;
    for (const start of order) {
      if (used[start]) continue;
      const path = [start];
      used[start] = true;
      const target = 2 + ((rng() * (maxLen - 1)) | 0);
      while (path.length < target) {
        const cur = path[path.length - 1];
        const cx = cur % n, cy = (cur / n) | 0;
        const cand = [];
        for (const [dx, dy] of [[1,0],[-1,0],[0,1],[0,-1]]) {
          const nx = cx + dx, ny = cy + dy;
          if (nx < 0 || nx >= n || ny < 0 || ny >= n) continue;
          const ni = ny * n + nx;
          if (!used[ni]) cand.push(ni);
        }
        if (!cand.length) break;
        const pick = cand[(rng() * cand.length) | 0];
        used[pick] = true;
        path.push(pick);
      }
      if (path.length < 2) { ok = false; break; }
      paths.push(path);
    }
    if (ok && used.every(Boolean)) return paths;
  }
  return null;
}

// ---- uniqueness solver -------------------------------------------------
// Backtrack thermometer by thermometer, trying each fill amount 0..length.
// Prune with a bidirectional bound: a line must never be over-filled, and
// must still be reachable from the cells the remaining thermometers hold.
function solveCount(n, thermos, rc, cc, limit) {
  const m = thermos.length;
  const rowCnt = thermos.map(t => { const a = new Array(n).fill(0); for (const c of t) a[(c / n) | 0]++; return a; });
  const colCnt = thermos.map(t => { const a = new Array(n).fill(0); for (const c of t) a[c % n]++; return a; });
  const sufR = [], sufC = [];
  for (let ti = 0; ti <= m; ti++) { sufR.push(new Array(n).fill(0)); sufC.push(new Array(n).fill(0)); }
  for (let ti = m - 1; ti >= 0; ti--) for (let i = 0; i < n; i++) {
    sufR[ti][i] = sufR[ti + 1][i] + rowCnt[ti][i];
    sufC[ti][i] = sufC[ti + 1][i] + colCnt[ti][i];
  }
  const rowS = new Array(n).fill(0), colS = new Array(n).fill(0);
  let count = 0;
  function apply(ti, k, d) {
    const t = thermos[ti];
    for (let j = 0; j < k; j++) { const c = t[j]; rowS[(c / n) | 0] += d; colS[c % n] += d; }
  }
  function rec(ti) {
    if (count >= limit) return;
    if (ti === m) {
      for (let i = 0; i < n; i++) if (rowS[i] !== rc[i] || colS[i] !== cc[i]) return;
      count++;
      return;
    }
    for (let i = 0; i < n; i++) {
      if (rowS[i] > rc[i] || rowS[i] + sufR[ti][i] < rc[i]) return;
      if (colS[i] > cc[i] || colS[i] + sufC[ti][i] < cc[i]) return;
    }
    const len = thermos[ti].length;
    for (let k = 0; k <= len; k++) {
      apply(ti, k, 1);
      let ok = true;
      for (let i = 0; i < n; i++) if (rowS[i] > rc[i] || colS[i] > cc[i]) { ok = false; break; }
      if (ok) rec(ti + 1);
      apply(ti, k, -1);
      if (count >= limit) return;
    }
  }
  rec(0);
  return count;
}

// ---- counts ------------------------------------------------------------
function lineCounts(n, thermos, marks) {
  const rowS = new Array(n).fill(0), colS = new Array(n).fill(0);
  for (let ti = 0; ti < thermos.length; ti++) {
    const t = thermos[ti];
    for (let j = 0; j < marks[ti]; j++) { const c = t[j]; rowS[(c / n) | 0]++; colS[c % n]++; }
  }
  return { rowS, colS };
}

// Cells lying in an over-filled row or column - the live red highlight.
function findConflicts(n, thermos, marks, rc, cc) {
  const bad = new Set();
  const { rowS, colS } = lineCounts(n, thermos, marks);
  const overRow = new Array(n).fill(false), overCol = new Array(n).fill(false);
  for (let i = 0; i < n; i++) { overRow[i] = rowS[i] > rc[i]; overCol[i] = colS[i] > cc[i]; }
  for (let ti = 0; ti < thermos.length; ti++) {
    const t = thermos[ti];
    for (let j = 0; j < marks[ti]; j++) {
      const c = t[j], y = (c / n) | 0, x = c % n;
      if (overRow[y] || overCol[x]) bad.add(c);
    }
  }
  return bad;
}

function isSolved(n, thermos, marks, rc, cc) {
  const { rowS, colS } = lineCounts(n, thermos, marks);
  for (let i = 0; i < n; i++) if (rowS[i] !== rc[i] || colS[i] !== cc[i]) return false;
  return true;
}

// ---- build -------------------------------------------------------------
function buildPuzzle(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const n = cfg.n;
  for (let attempt = 0; attempt < 400; attempt++) {
    const rng = seededRandom(cfg.seed + attempt * 1009);
    let thermos = pathCover(n, rng, n);
    if (!thermos) continue;
    // Longest thermometer first - tighter pruning for the solver.
    thermos = thermos.slice().sort((a, b) => b.length - a.length);
    const fills = thermos.map(t => (rng() * (t.length + 1)) | 0);
    const rc = new Array(n).fill(0), cc = new Array(n).fill(0);
    for (let ti = 0; ti < thermos.length; ti++) {
      const t = thermos[ti];
      for (let j = 0; j < fills[ti]; j++) { const c = t[j]; rc[(c / n) | 0]++; cc[c % n]++; }
    }
    const tot = fills.reduce((a, b) => a + b, 0);
    if (tot <= n || tot >= n * n - n) continue;          // reject near-empty / near-full
    if (solveCount(n, thermos, rc, cc, 2) !== 1) continue;
    const owner = new Array(n * n).fill(-1);
    const pos = new Array(n * n).fill(-1);
    for (let ti = 0; ti < thermos.length; ti++) {
      thermos[ti].forEach((c, j) => { owner[c] = ti; pos[c] = j; });
    }
    return { n, thermos, fills, rc, cc, owner, pos, levelIndex, cfg };
  }
  return null;
}
