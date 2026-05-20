// Pixel Kuromasu - a "visible cells" logic puzzle. Shade cells black so each
// numbered cell sees exactly that many cells (itself included) before hitting
// a black cell or the grid edge. No two black cells touch orthogonally, and
// every white cell must connect.
//
// Each level scatters a random black placement, derives the full hint set
// from it, then trims hints while a backtracking solver keeps the puzzle
// uniquely solvable.

const VW = 360, VH = 480;

// fixed cell types
const F_BLANK = 0, F_HINT = 1;
// player marks on blank cells
const PB_BLANK = 0, PB_BLACK = 1, PB_WHITE = 2;

const LEVELS = [
  { name: ['Hamlet', '小村'], seed: 14,  n: 5 },
  { name: ['Town', '城镇'],   seed: 47,  n: 5 },
  { name: ['Quarter', '街区'], seed: 96, n: 6 },
  { name: ['Plaza', '广场'],   seed: 162, n: 6 },
  { name: ['Citadel', '城堡'], seed: 247, n: 6 },
  { name: ['Capital', '都城'], seed: 355, n: 6 },
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

const ix = (n, r, c) => r * n + c;
const ORTH = [[-1, 0], [1, 0], [0, -1], [0, 1]];
function inBounds(n, r, c) { return r >= 0 && c >= 0 && r < n && c < n; }

// ---- visible-count and constraint helpers --------------------------------
// Count white cells visible (self + each dir until a black or edge) given a
// boolean grid where black[i] === 1 means BLACK.
function visCount(black, n, r, c) {
  let count = 1;
  for (const [dr, dc] of ORTH) {
    let nr = r + dr, nc = c + dc;
    while (nr >= 0 && nc >= 0 && nr < n && nc < n && !black[ix(n, nr, nc)]) {
      count++;
      nr += dr; nc += dc;
    }
  }
  return count;
}

// Are all WHITE cells (black[i]===0) connected (4-adj)?
function whitesConnected(black, n) {
  const N = n * n;
  let first = -1, total = 0;
  for (let i = 0; i < N; i++) if (!black[i]) { total++; if (first < 0) first = i; }
  if (first < 0) return false;
  const seen = new Uint8Array(N);
  const stack = [first];
  seen[first] = 1;
  let cnt = 0;
  while (stack.length) {
    const cur = stack.pop();
    cnt++;
    const r = (cur / n) | 0, c = cur % n;
    for (const [dr, dc] of ORTH) {
      const nr = r + dr, nc = c + dc;
      if (!inBounds(n, nr, nc)) continue;
      const ni = ix(n, nr, nc);
      if (seen[ni] || black[ni]) continue;
      seen[ni] = 1; stack.push(ni);
    }
  }
  return cnt === total;
}

// No two black cells orthogonally adjacent?
function blacksValid(black, n) {
  for (let i = 0; i < n * n; i++) {
    if (!black[i]) continue;
    const r = (i / n) | 0, c = i % n;
    for (const [dr, dc] of ORTH) {
      const nr = r + dr, nc = c + dc;
      if (!inBounds(n, nr, nc)) continue;
      if (black[ix(n, nr, nc)]) return false;
    }
  }
  return true;
}

// ---- generation ----------------------------------------------------------
function placeBlacks(n, target, rng) {
  const black = new Uint8Array(n * n);
  const order = [];
  for (let i = 0; i < n * n; i++) order.push(i);
  for (let k = order.length - 1; k > 0; k--) {
    const j = (rng() * (k + 1)) | 0;
    [order[k], order[j]] = [order[j], order[k]];
  }
  let placed = 0;
  for (const i of order) {
    if (placed >= target) break;
    const r = (i / n) | 0, c = i % n;
    let ok = true;
    for (const [dr, dc] of ORTH) {
      const nr = r + dr, nc = c + dc;
      if (!inBounds(n, nr, nc)) continue;
      if (black[ix(n, nr, nc)]) { ok = false; break; }
    }
    if (!ok) continue;
    black[i] = 1;
    placed++;
  }
  return black;
}

// ---- solver --------------------------------------------------------------
// Count solutions consistent with the hint cells. `hints` is a map "i" ->
// required visible count. Cells listed in `hints` are forced WHITE. All other
// cells are blank and the solver assigns B/W. Stops at `limit`.
function solveCount(n, hints, limit) {
  const N = n * n;
  const black = new Uint8Array(N);
  const isHint = new Uint8Array(N);
  for (const k in hints) isHint[+k] = 1;
  const cells = [];
  for (let i = 0; i < N; i++) if (!isHint[i]) cells.push(i);
  let found = 0;

  function bt(k) {
    if (found >= limit) return;
    if (k === cells.length) {
      // every hint's visible count must match
      for (const sk in hints) {
        const j = +sk;
        if (visCount(black, n, (j / n) | 0, j % n) !== hints[sk]) return;
      }
      // every white cell connected
      if (!whitesConnected(black, n)) return;
      found++;
      return;
    }
    const i = cells[k];
    // try WHITE
    bt(k + 1);
    if (found >= limit) return;
    // try BLACK if no adjacent black yet
    const r = (i / n) | 0, c = i % n;
    let ok = true;
    for (const [dr, dc] of ORTH) {
      const nr = r + dr, nc = c + dc;
      if (!inBounds(n, nr, nc)) continue;
      if (black[ix(n, nr, nc)]) { ok = false; break; }
    }
    if (ok) {
      black[i] = 1;
      bt(k + 1);
      black[i] = 0;
    }
  }
  bt(0);
  return found;
}

// Build a uniquely-solvable Kuromasu.
function buildPuzzle(level) {
  const n = level.n;
  const rng = seededRandom(level.seed);
  const target = Math.max(2, Math.floor(n * n * 0.2));
  for (let attempt = 0; attempt < 600; attempt++) {
    const black = placeBlacks(n, target, rng);
    if (!blacksValid(black, n)) continue;
    if (!whitesConnected(black, n)) continue;
    // start with every white cell as a hint
    const hints = {};
    for (let i = 0; i < n * n; i++) if (!black[i]) hints[i] = visCount(black, n, (i / n) | 0, i % n);
    if (solveCount(n, hints, 2) !== 1) continue;
    // greedily drop hints while uniqueness holds
    const hintIdxs = Object.keys(hints).map(Number);
    for (let k = hintIdxs.length - 1; k > 0; k--) {
      const j = (rng() * (k + 1)) | 0;
      [hintIdxs[k], hintIdxs[j]] = [hintIdxs[j], hintIdxs[k]];
    }
    for (const i of hintIdxs) {
      const saved = hints[i];
      delete hints[i];
      if (solveCount(n, hints, 2) !== 1) hints[i] = saved;
    }
    return { n, black, hints };
  }
  return null;
}

// ---- evaluation (game side) ---------------------------------------------
function evaluate(pz, marks) {
  const n = pz.n, N = n * n;
  const bad = new Set();
  // turn player marks + hints into a B/W grid (unknowns count as white)
  const black = new Uint8Array(N);
  for (let i = 0; i < N; i++) if (marks[i] === PB_BLACK) black[i] = 1;
  // adjacent-black violation
  for (let i = 0; i < N; i++) {
    if (!black[i]) continue;
    const r = (i / n) | 0, c = i % n;
    for (const [dr, dc] of ORTH) {
      const nr = r + dr, nc = c + dc;
      if (!inBounds(n, nr, nc)) continue;
      const j = ix(n, nr, nc);
      if (black[j]) { bad.add(i); bad.add(j); }
    }
  }
  // hint cells cannot be marked black
  for (const sk in pz.hints) if (marks[+sk] === PB_BLACK) bad.add(+sk);
  // for solve check: hints must match AND whites connected AND no bad
  let solved = bad.size === 0;
  if (solved) {
    for (const sk in pz.hints) {
      const j = +sk;
      if (visCount(black, n, (j / n) | 0, j % n) !== pz.hints[sk]) { solved = false; break; }
    }
  }
  if (solved && !whitesConnected(black, n)) solved = false;
  return { bad, solved };
}

function cycleMark(v) { return (v + 1) % 3; }
