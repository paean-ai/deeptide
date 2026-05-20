// Pixel Suguru - Suguru (a.k.a. Tectonic / Sukibun) number puzzle.
//
// Rules:
//   * The grid is partitioned into irregular regions of 1..5 cells.
//   * Each region of size n contains the digits 1..n exactly once.
//   * The same digit cannot touch itself - even diagonally.
//   * Solve the unique completion from the given clues.
//
// Each level is { n, seed, minClues }. buildPuzzle is deterministic in seed
// and yields { regions, solution, clues } so a puzzle is the same every run.

const VW = 360, VH = 480;

const LEVELS = [
  { n: 5, seed: 131,  minClues: 5  },
  { n: 5, seed: 281,  minClues: 6  },
  { n: 5, seed: 457,  minClues: 7  },
  { n: 6, seed: 602,  minClues: 10 },
  { n: 6, seed: 813,  minClues: 11 },
  { n: 6, seed: 1117, minClues: 12 },
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

// ---- generation ---------------------------------------------------------
// BFS-grow random regions of target size 2..5; merge any leftover singletons
// into the smallest adjacent neighbour so the board is made of fat shapes.
function partition(n, rng) {
  const total = n * n;
  const owner = new Array(total).fill(-1);
  const regions = [];
  while (owner.some(v => v === -1)) {
    const empties = [];
    for (let i = 0; i < total; i++) if (owner[i] === -1) empties.push(i);
    const start = empties[(rng() * empties.length) | 0];
    const target = 2 + ((rng() * 4) | 0);
    const cells = [start];
    owner[start] = regions.length;
    while (cells.length < target) {
      const cand = [];
      for (const c of cells) {
        const r = (c / n) | 0, col = c % n;
        for (const [nr, nc] of [[r-1,col],[r+1,col],[r,col-1],[r,col+1]]) {
          if (nr<0||nr>=n||nc<0||nc>=n) continue;
          const ni = nr * n + nc;
          if (owner[ni] === -1) cand.push(ni);
        }
      }
      if (!cand.length) break;
      const pick = cand[(rng() * cand.length) | 0];
      cells.push(pick);
      owner[pick] = regions.length;
    }
    regions.push(cells);
  }
  // Merge singletons.
  for (let i = regions.length - 1; i >= 0; i--) {
    if (regions[i].length !== 1) continue;
    const c = regions[i][0];
    const r = (c / n) | 0, col = c % n;
    let bestReg = -1, bestSize = 99;
    for (const [nr, nc] of [[r-1,col],[r+1,col],[r,col-1],[r,col+1]]) {
      if (nr<0||nr>=n||nc<0||nc>=n) continue;
      const reg = owner[nr * n + nc];
      if (reg === i) continue;
      if (regions[reg].length < bestSize && regions[reg].length < 5) {
        bestSize = regions[reg].length; bestReg = reg;
      }
    }
    if (bestReg < 0) continue;
    regions[bestReg].push(c);
    owner[c] = bestReg;
    regions[i].length = 0;
  }
  const cleaned = regions.filter(r => r.length);
  cleaned.forEach((r, id) => r.forEach(c => owner[c] = id));
  // Reject partitions with any region outside [2,5] - retry with a fresh seed.
  if (cleaned.some(r => r.length > 5 || r.length < 2)) return null;
  return cleaned;
}

// Fill the regions with values respecting both region-distinct and the
// 8-adjacent rule. Returns the solution array, or null if impossible.
function fillRegions(n, regions, rng) {
  const cellReg = new Array(n * n);
  regions.forEach((r, id) => r.forEach(c => cellReg[c] = id));
  const regSize = regions.map(r => r.length);
  const sol = new Array(n * n).fill(0);
  const used = regions.map(() => new Set());
  function adjOk(c, v) {
    const r = (c / n) | 0, col = c % n;
    for (let dr = -1; dr <= 1; dr++) for (let dc = -1; dc <= 1; dc++) {
      if (!dr && !dc) continue;
      const nr = r + dr, nc = col + dc;
      if (nr<0||nr>=n||nc<0||nc>=n) continue;
      if (sol[nr * n + nc] === v) return false;
    }
    return true;
  }
  function rec(idx) {
    if (idx === n * n) return true;
    const reg = cellReg[idx], size = regSize[reg];
    const vs = [];
    for (let v = 1; v <= size; v++) if (!used[reg].has(v) && adjOk(idx, v)) vs.push(v);
    for (let i = vs.length - 1; i > 0; i--) {
      const j = (rng() * (i + 1)) | 0;
      [vs[i], vs[j]] = [vs[j], vs[i]];
    }
    for (const v of vs) {
      sol[idx] = v;
      used[reg].add(v);
      if (rec(idx + 1)) return true;
      used[reg].delete(v);
      sol[idx] = 0;
    }
    return false;
  }
  return rec(0) ? sol.slice() : null;
}

// Count up to `limit` solutions of (regions, clues). Most-constrained-cell
// first to converge quickly.
function solveCount(n, regions, clues, limit) {
  const cellReg = new Array(n * n);
  regions.forEach((r, id) => r.forEach(c => cellReg[c] = id));
  const regSize = regions.map(r => r.length);
  const g = clues.slice();
  const used = regions.map(r => {
    const s = new Set();
    for (const c of r) if (g[c]) s.add(g[c]);
    return s;
  });
  let cnt = 0;
  function adjOk(c, v) {
    const r = (c / n) | 0, col = c % n;
    for (let dr = -1; dr <= 1; dr++) for (let dc = -1; dc <= 1; dc++) {
      if (!dr && !dc) continue;
      const nr = r + dr, nc = col + dc;
      if (nr<0||nr>=n||nc<0||nc>=n) continue;
      if (g[nr * n + nc] === v) return false;
    }
    return true;
  }
  function pickCell() {
    let best = -1, bc = Infinity;
    for (let i = 0; i < n * n; i++) {
      if (g[i]) continue;
      const reg = cellReg[i], size = regSize[reg];
      let c = 0;
      for (let v = 1; v <= size; v++) {
        if (used[reg].has(v)) continue;
        if (!adjOk(i, v)) continue;
        c++;
      }
      if (c < bc) { bc = c; best = i; if (c <= 1) break; }
    }
    return best;
  }
  function rec() {
    if (cnt >= limit) return;
    const c = pickCell();
    if (c === -1) { cnt++; return; }
    const reg = cellReg[c], size = regSize[reg];
    for (let v = 1; v <= size; v++) {
      if (used[reg].has(v)) continue;
      if (!adjOk(c, v)) continue;
      g[c] = v;
      used[reg].add(v);
      rec();
      g[c] = 0;
      used[reg].delete(v);
      if (cnt >= limit) return;
    }
  }
  rec();
  return cnt;
}

// Greedy trim: remove givens in random order while uniqueness holds. Stop
// when the puzzle is at minClues so a solver-blank board never appears in
// front of a player.
function trimClues(n, regions, sol, rng, minClues) {
  const clues = sol.slice();
  const order = [];
  for (let i = 0; i < n * n; i++) order.push(i);
  for (let i = order.length - 1; i > 0; i--) {
    const j = (rng() * (i + 1)) | 0;
    [order[i], order[j]] = [order[j], order[i]];
  }
  for (const i of order) {
    const filled = clues.filter(v => v).length;
    if (filled <= minClues) break;
    const v = clues[i];
    clues[i] = 0;
    if (solveCount(n, regions, clues, 2) !== 1) clues[i] = v;
  }
  return clues;
}

function buildPuzzle(levelIndex) {
  const cfg = LEVELS[levelIndex];
  for (let attempt = 0; attempt < 80; attempt++) {
    const rng = seededRandom(cfg.seed + attempt * 1009);
    const regs = partition(cfg.n, rng);
    if (!regs) continue;
    const sol = fillRegions(cfg.n, regs, rng);
    if (!sol) continue;
    const clues = trimClues(cfg.n, regs, sol, rng, cfg.minClues);
    return { n: cfg.n, regions: regs, solution: sol, clues, levelIndex };
  }
  return null;
}

// ---- live validation (for UI red-flag) ---------------------------------
// Returns a set of cell indices that currently violate Suguru rules in the
// player's grid (region duplicate or 8-adjacent duplicate). Empty cells are
// never flagged.
function findViolations(n, regions, grid) {
  const bad = new Set();
  const cellReg = new Array(n * n);
  regions.forEach((r, id) => r.forEach(c => cellReg[c] = id));
  // Region duplicates.
  for (const r of regions) {
    const seen = new Map();
    for (const c of r) {
      const v = grid[c];
      if (!v) continue;
      if (seen.has(v)) { bad.add(c); bad.add(seen.get(v)); }
      else seen.set(v, c);
    }
  }
  // 8-adjacent duplicates.
  for (let c = 0; c < n * n; c++) {
    const v = grid[c];
    if (!v) continue;
    const r = (c / n) | 0, col = c % n;
    for (let dr = -1; dr <= 1; dr++) for (let dc = -1; dc <= 1; dc++) {
      if (!dr && !dc) continue;
      const nr = r + dr, nc = col + dc;
      if (nr<0||nr>=n||nc<0||nc>=n) continue;
      const ni = nr * n + nc;
      if (grid[ni] === v) { bad.add(c); bad.add(ni); }
    }
  }
  return bad;
}

function isSolved(n, regions, grid) {
  for (let i = 0; i < n * n; i++) if (!grid[i]) return false;
  if (findViolations(n, regions, grid).size) return false;
  // Each region must contain 1..size.
  for (const r of regions) {
    const expect = new Set();
    for (let v = 1; v <= r.length; v++) expect.add(v);
    for (const c of r) expect.delete(grid[c]);
    if (expect.size) return false;
  }
  return true;
}
