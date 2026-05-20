// Pixel Nurikabe - island/sea logic puzzle.
//
// Rules:
//   * Numbered cells are island "heads"; each number is the size of the
//     4-connected white "island" it belongs to.
//   * Every island contains exactly one numbered head.
//   * Different islands cannot share an orthogonal edge.
//   * All remaining cells are "sea" (shaded). The sea must be one connected
//     orthogonal region with no fully-sea 2x2 block.
//
// Each level is { n, sizes, seed }. buildPuzzle is deterministic in seed and
// returns { clues, solution } where clues are island heads with size labels.

const VW = 360, VH = 480;

const LEVELS = [
  { n: 5, sizes: [2, 3, 2],          seed: 101 },
  { n: 5, sizes: [1, 4, 2],          seed: 203 },
  { n: 5, sizes: [3, 2, 2, 1],       seed: 307 },
  { n: 6, sizes: [4, 3, 2, 1],       seed: 509 },
  { n: 6, sizes: [3, 2, 2, 2, 1],    seed: 612 },
  { n: 6, sizes: [3, 3, 2, 2],       seed: 500 },
];
const LEVEL_COUNT = LEVELS.length;

// Cell tags inside the solver / live board:
const UNKNOWN = 0;
const SEA     = 1;
//   island k => k + 2

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () {
    s = (s * 16807) % 2147483647;
    return (s - 1) / 2147483646;
  };
}

function idx(n, x, y) { return y * n + x; }

// ---- solution construction ----------------------------------------------
// Place each island in turn: pick a head not adjacent to other islands, grow
// orthogonally without ever touching another island, until the island reaches
// its size. Reject the attempt if any step is blocked or the resulting sea
// fails the "single connected region with no 2x2" rule.
function buildSolution(n, sizes, rng) {
  for (let attempt = 0; attempt < 400; attempt++) {
    const g = new Array(n * n).fill(0);
    const islands = [];
    let ok = true;
    for (const s of sizes) {
      const candidates = [];
      for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) {
        if (g[idx(n, x, y)]) continue;
        let adjOk = true;
        for (const [dx, dy] of [[1,0],[-1,0],[0,1],[0,-1]]) {
          const nx = x + dx, ny = y + dy;
          if (nx<0||nx>=n||ny<0||ny>=n) continue;
          if (g[idx(n, nx, ny)] >= 2) { adjOk = false; break; }
        }
        if (adjOk) candidates.push([x, y]);
      }
      if (!candidates.length) { ok = false; break; }
      const [hx, hy] = candidates[(rng() * candidates.length) | 0];
      const cells = [[hx, hy]];
      const tag = islands.length + 2;
      g[idx(n, hx, hy)] = tag;
      while (cells.length < s) {
        const grow = [];
        for (const [cx, cy] of cells) {
          for (const [dx, dy] of [[1,0],[-1,0],[0,1],[0,-1]]) {
            const nx = cx + dx, ny = cy + dy;
            if (nx<0||nx>=n||ny<0||ny>=n) continue;
            if (g[idx(n, nx, ny)]) continue;
            let adjOk = true;
            for (const [dx2, dy2] of [[1,0],[-1,0],[0,1],[0,-1]]) {
              const ax = nx + dx2, ay = ny + dy2;
              if (ax<0||ax>=n||ay<0||ay>=n) continue;
              const v = g[idx(n, ax, ay)];
              if (v >= 2 && v !== tag) { adjOk = false; break; }
            }
            if (adjOk) grow.push([nx, ny]);
          }
        }
        if (!grow.length) { ok = false; break; }
        const [nx, ny] = grow[(rng() * grow.length) | 0];
        cells.push([nx, ny]);
        g[idx(n, nx, ny)] = tag;
      }
      if (!ok || cells.length < s) { ok = false; break; }
      islands.push({ head: [hx, hy], size: s, cells: cells.slice() });
    }
    if (!ok) continue;
    for (let i = 0; i < n * n; i++) if (!g[i]) g[i] = SEA;
    if (!seaValid(n, g)) continue;
    return { g, islands };
  }
  return null;
}

// Sea = single orthogonal component + no 2x2 block fully shaded.
function seaValid(n, g) {
  for (let y = 0; y < n - 1; y++) for (let x = 0; x < n - 1; x++) {
    if (g[idx(n, x, y)] === SEA && g[idx(n, x+1, y)] === SEA
     && g[idx(n, x, y+1)] === SEA && g[idx(n, x+1, y+1)] === SEA) return false;
  }
  let start = -1, total = 0;
  for (let i = 0; i < n * n; i++) if (g[i] === SEA) {
    if (start < 0) start = i;
    total++;
  }
  if (start < 0) return true;
  const seen = new Uint8Array(n * n);
  seen[start] = 1;
  const q = [start];
  let cnt = 1;
  while (q.length) {
    const v = q.shift();
    const x = v % n, y = (v / n) | 0;
    for (const [dx, dy] of [[1,0],[-1,0],[0,1],[0,-1]]) {
      const nx = x + dx, ny = y + dy;
      if (nx<0||nx>=n||ny<0||ny>=n) continue;
      const ni = idx(n, nx, ny);
      if (seen[ni] || g[ni] !== SEA) continue;
      seen[ni] = 1; q.push(ni); cnt++;
    }
  }
  return cnt === total;
}

// ---- shape-enumeration uniqueness verifier ------------------------------
// Enumerate every connected polyomino of `size` cells containing (hx, hy)
// that fits in n*n. This is the search space per island in the solver.
function enumShapes(n, hx, hy, size) {
  const out = [];
  const seen = new Set();
  function rec(cells) {
    if (cells.size === size) {
      const key = [...cells].sort((a, b) => a - b).join(',');
      if (seen.has(key)) return;
      seen.add(key);
      out.push([...cells]);
      return;
    }
    const frontier = new Set();
    for (const c of cells) {
      const x = c % n, y = (c / n) | 0;
      for (const [dx, dy] of [[1,0],[-1,0],[0,1],[0,-1]]) {
        const nx = x + dx, ny = y + dy;
        if (nx<0||nx>=n||ny<0||ny>=n) continue;
        const ni = idx(n, nx, ny);
        if (cells.has(ni)) continue;
        frontier.add(ni);
      }
    }
    for (const f of frontier) {
      cells.add(f); rec(cells); cells.delete(f);
    }
  }
  rec(new Set([idx(n, hx, hy)]));
  return out;
}

// Count solutions of the puzzle defined by `clues`, up to `limit`.
// Pick shapes for each island in ascending shape-set size (fastest pruning),
// then check the leftover cells form a valid sea.
function solveCount(n, clues, limit) {
  const shapesPer = clues.map(c => enumShapes(n, c.x, c.y, c.size));
  const order = clues.map((_, i) => i).sort((a, b) => shapesPer[a].length - shapesPer[b].length);
  let count = 0;
  const occupied = new Array(n * n).fill(false);
  function rec(p) {
    if (count >= limit) return;
    if (p === order.length) {
      const g = new Array(n * n).fill(SEA);
      for (let i = 0; i < n * n; i++) if (occupied[i]) g[i] = 2;
      if (seaValid(n, g)) count++;
      return;
    }
    const k = order[p];
    for (const shape of shapesPer[k]) {
      let ok = true;
      for (const c of shape) if (occupied[c]) { ok = false; break; }
      if (!ok) continue;
      for (const c of shape) {
        const x = c % n, y = (c / n) | 0;
        for (const [dx, dy] of [[1,0],[-1,0],[0,1],[0,-1]]) {
          const nx = x + dx, ny = y + dy;
          if (nx<0||nx>=n||ny<0||ny>=n) continue;
          const ni = idx(n, nx, ny);
          if (occupied[ni] && !shape.includes(ni)) { ok = false; break; }
        }
        if (!ok) break;
      }
      if (!ok) continue;
      for (const c of shape) occupied[c] = true;
      rec(p + 1);
      for (const c of shape) occupied[c] = false;
      if (count >= limit) return;
    }
  }
  rec(0);
  return count;
}

function buildPuzzle(levelIndex) {
  const cfg = LEVELS[levelIndex];
  for (let attempt = 0; attempt < 300; attempt++) {
    const rng = seededRandom(cfg.seed + attempt * 1009);
    const sol = buildSolution(cfg.n, cfg.sizes, rng);
    if (!sol) continue;
    const clues = sol.islands.map(i => ({ x: i.head[0], y: i.head[1], size: i.size }));
    if (solveCount(cfg.n, clues, 2) === 1) {
      return { n: cfg.n, clues, solution: sol.g, levelIndex };
    }
  }
  return null;
}

// ---- live validation (for UI red-flag) ---------------------------------
// Live red highlights while the player solves:
//   * Two different island tags adjacent.
//   * Any 2x2 of marked-sea cells.
//   * An island with more cells than its clue.
// `grid` is the live tag grid: 0/UNKNOWN, SEA, or k+2 for island k.
function findViolations(n, clues, grid) {
  const bad = new Set();
  // Different islands touching.
  for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) {
    const a = grid[idx(n, x, y)];
    if (a < 2) continue;
    for (const [dx, dy] of [[1,0],[0,1]]) {
      const nx = x + dx, ny = y + dy;
      if (nx>=n||ny>=n) continue;
      const ni = idx(n, nx, ny);
      const b = grid[ni];
      if (b >= 2 && b !== a) { bad.add(idx(n, x, y)); bad.add(ni); }
    }
  }
  // 2x2 sea pool.
  for (let y = 0; y < n - 1; y++) for (let x = 0; x < n - 1; x++) {
    if (grid[idx(n, x, y)] === SEA && grid[idx(n, x+1, y)] === SEA
     && grid[idx(n, x, y+1)] === SEA && grid[idx(n, x+1, y+1)] === SEA) {
      bad.add(idx(n, x, y));
      bad.add(idx(n, x+1, y));
      bad.add(idx(n, x, y+1));
      bad.add(idx(n, x+1, y+1));
    }
  }
  // Oversized islands.
  for (let k = 0; k < clues.length; k++) {
    const start = idx(n, clues[k].x, clues[k].y);
    const seen = new Set([start]);
    const q = [start];
    while (q.length) {
      const v = q.shift();
      const x = v % n, y = (v / n) | 0;
      for (const [dx, dy] of [[1,0],[-1,0],[0,1],[0,-1]]) {
        const nx = x + dx, ny = y + dy;
        if (nx<0||nx>=n||ny<0||ny>=n) continue;
        const ni = idx(n, nx, ny);
        if (seen.has(ni)) continue;
        if (grid[ni] === k + 2) { seen.add(ni); q.push(ni); }
      }
    }
    if (seen.size > clues[k].size) for (const c of seen) bad.add(c);
  }
  return bad;
}

function isSolved(n, clues, grid) {
  for (let i = 0; i < n * n; i++) if (grid[i] === UNKNOWN) return false;
  if (findViolations(n, clues, grid).size) return false;
  // Each island = exactly clue size, all matching tag cells reached from head.
  for (let k = 0; k < clues.length; k++) {
    const start = idx(n, clues[k].x, clues[k].y);
    const seen = new Set([start]);
    const q = [start];
    while (q.length) {
      const v = q.shift();
      const x = v % n, y = (v / n) | 0;
      for (const [dx, dy] of [[1,0],[-1,0],[0,1],[0,-1]]) {
        const nx = x + dx, ny = y + dy;
        if (nx<0||nx>=n||ny<0||ny>=n) continue;
        const ni = idx(n, nx, ny);
        if (seen.has(ni)) continue;
        if (grid[ni] === k + 2) { seen.add(ni); q.push(ni); }
      }
    }
    if (seen.size !== clues[k].size) return false;
    let total = 0;
    for (let i = 0; i < n * n; i++) if (grid[i] === k + 2) total++;
    if (total !== clues[k].size) return false;
  }
  return seaValid(n, grid);
}
