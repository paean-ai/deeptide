// Pixel Norinori - Nikoli domino-shading logic puzzle.
//
// Rules:
//   * The grid is partitioned into regions of >= 2 cells.
//   * Shade exactly two cells per region, and they MUST be orthogonally
//     adjacent (so each region holds exactly one domino).
//   * Two dominoes from DIFFERENT regions may not share an edge — they may
//     touch at a corner, but never along a side.
//   * A few cells start revealed as hints: a shaded hint must end up
//     shaded; an unshaded hint must end up unshaded.
//
// Each level is { n, k, seed }. buildPuzzle is deterministic per seed.

const VW = 360, VH = 480;

const LEVELS = [
  { name: ['Shoal',   '浅滩'], n: 5, k: 5, seed: 131 },
  { name: ['Reef',    '礁石'], n: 5, k: 5, seed: 287 },
  { name: ['Lagoon',  '潟湖'], n: 6, k: 6, seed: 431 },
  { name: ['Sound',   '海峡'], n: 6, k: 7, seed: 531 },
  { name: ['Trench',  '海沟'], n: 7, k: 7, seed: 631 },
  { name: ['Abyss',   '深渊'], n: 7, k: 8, seed: 733 },
];
const LEVEL_COUNT = LEVELS.length;

// Live grid tags:
const UNKNOWN  = -1;
const UNSHADED = 0;
const SHADED   = 1;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () {
    s = (s * 16807) % 2147483647;
    return (s - 1) / 2147483646;
  };
}

// ---- partition (round-robin BFS-expansion) -----------------------------
function partition(n, k, rng) {
  const total = n * n;
  const seeds = new Set();
  while (seeds.size < k) seeds.add((rng() * total) | 0);
  const owner = new Array(total).fill(-1);
  const fronts = [];
  let id = 0;
  for (const s of seeds) { owner[s] = id; fronts.push([s]); id++; }
  let remaining = total - k;
  while (remaining > 0) {
    let progressed = false;
    for (let r = 0; r < k; r++) {
      if (!fronts[r].length) continue;
      const cands = [];
      for (const c of fronts[r]) {
        const x = c % n, y = (c / n) | 0;
        for (const [dx, dy] of [[1,0],[-1,0],[0,1],[0,-1]]) {
          const nx = x + dx, ny = y + dy;
          if (nx<0||nx>=n||ny<0||ny>=n) continue;
          const ni = ny * n + nx;
          if (owner[ni] === -1) cands.push(ni);
        }
      }
      if (!cands.length) { fronts[r].length = 0; continue; }
      const pick = cands[(rng() * cands.length) | 0];
      owner[pick] = r;
      fronts[r].push(pick);
      remaining--;
      progressed = true;
      if (remaining <= 0) break;
    }
    if (!progressed) return null;
  }
  const regions = Array.from({ length: k }, () => []);
  for (let i = 0; i < total; i++) regions[owner[i]].push(i);
  for (const r of regions) if (r.length < 2) return null;
  return { regions, owner };
}

// For each region, enumerate every orthogonal cell-pair contained in it.
function regionDominoes(regions, n) {
  return regions.map(reg => {
    const set = new Set(reg);
    const doms = [];
    for (const c of reg) {
      for (const [dx, dy] of [[1, 0], [0, 1]]) {
        const x = c % n, y = (c / n) | 0;
        const nx = x + dx, ny = y + dy;
        if (nx<0||nx>=n||ny<0||ny>=n) continue;
        const ni = ny * n + nx;
        if (set.has(ni)) doms.push([c, ni]);
      }
    }
    return doms;
  });
}

// True if any cell of a is orthogonally adjacent to any cell of b (and
// they're not the same domino).
function touchOrth(n, a, b) {
  for (const c of a) {
    const x = c % n, y = (c / n) | 0;
    for (const [dx, dy] of [[1,0],[-1,0],[0,1],[0,-1]]) {
      const nx = x + dx, ny = y + dy;
      if (nx<0||nx>=n||ny<0||ny>=n) continue;
      const ni = ny * n + nx;
      if (b.includes(ni)) return true;
    }
  }
  return false;
}

// ---- uniqueness solver -------------------------------------------------
// Backtrack region by region, picking one domino per region; prune by hints
// (per-region) and by the no-touching rule against previously-placed
// dominoes. At the leaf, count it as a solution.
function solveCount(n, regions, hints, limit) {
  const doms = regionDominoes(regions, n);
  let count = 0;
  const placed = new Array(regions.length).fill(null);
  function rec(idx) {
    if (count >= limit) return;
    if (idx === regions.length) { count++; return; }
    for (const d of doms[idx]) {
      // Touching previous dominoes?
      let ok = true;
      for (let j = 0; j < idx; j++) {
        if (touchOrth(n, d, placed[j])) { ok = false; break; }
      }
      if (!ok) continue;
      // Per-region hint check: shaded hints must be in d; unshaded must not.
      let hintOk = true;
      for (const c of regions[idx]) {
        const inDom = d.includes(c);
        if (hints[c] === SHADED   && !inDom) { hintOk = false; break; }
        if (hints[c] === UNSHADED &&  inDom) { hintOk = false; break; }
      }
      if (!hintOk) continue;
      placed[idx] = d;
      rec(idx + 1);
      placed[idx] = null;
      if (count >= limit) return;
    }
  }
  rec(0);
  return count;
}

// Generate a valid solution + hint-trim down to minimum hints.
function makePuzzle(n, k, rng) {
  const p = partition(n, k, rng);
  if (!p) return null;
  const doms = regionDominoes(p.regions, n);
  for (const d of doms) if (d.length === 0) return null;
  // Pick a random valid arrangement.
  let chosen = null;
  const placed = new Array(p.regions.length).fill(null);
  function pick(idx) {
    if (chosen) return;
    if (idx === p.regions.length) { chosen = placed.slice(); return; }
    const ds = doms[idx].slice();
    for (let i = ds.length - 1; i > 0; i--) {
      const j = (rng() * (i + 1)) | 0;
      [ds[i], ds[j]] = [ds[j], ds[i]];
    }
    for (const d of ds) {
      let ok = true;
      for (let j = 0; j < idx; j++) {
        if (touchOrth(n, d, placed[j])) { ok = false; break; }
      }
      if (!ok) continue;
      placed[idx] = d;
      pick(idx + 1);
      placed[idx] = null;
      if (chosen) return;
    }
  }
  pick(0);
  if (!chosen) return null;
  const shade = new Array(n * n).fill(0);
  for (const d of chosen) for (const c of d) shade[c] = 1;
  // Trim hints from FULLY-REVEALED down to the smallest unique set.
  // NOTE: a fresh partition with random domino placement has very LOW
  // uniqueness yield (200 random 5x5 partitions tested -> 0 unique
  // without hints, 168 multi, 22 zero). Hint cells are required to
  // disambiguate; trim greedily for compactness.
  const hints = shade.slice();          // start = SHADED/UNSHADED everywhere
  const order = [];
  for (let i = 0; i < n * n; i++) order.push(i);
  for (let i = order.length - 1; i > 0; i--) {
    const j = (rng() * (i + 1)) | 0;
    [order[i], order[j]] = [order[j], order[i]];
  }
  for (const i of order) {
    const old = hints[i];
    hints[i] = UNKNOWN;
    if (solveCount(n, p.regions, hints, 2) !== 1) hints[i] = old;
  }
  return { n, regions: p.regions, dominoes: chosen, shade, hints };
}

function buildPuzzle(levelIndex) {
  const cfg = LEVELS[levelIndex];
  for (let attempt = 0; attempt < 200; attempt++) {
    const rng = seededRandom(cfg.seed + attempt * 1009);
    const p = makePuzzle(cfg.n, cfg.k, rng);
    if (p) return { ...p, levelIndex, cfg };
  }
  return null;
}

// ---- live validation ---------------------------------------------------
// Returns the set of cell indices currently in conflict in `marks` (-1/0/1).
//   * Two SHADED cells in different regions that share an edge.
//   * A region with > 2 SHADED cells.
//   * A region with exactly 2 SHADED cells that are NOT orthogonally
//     adjacent (i.e., not a domino).
function findViolations(n, regions, marks) {
  const bad = new Set();
  const owner = new Array(n * n);
  regions.forEach((r, id) => r.forEach(c => owner[c] = id));
  // Cross-region edge touch.
  for (let i = 0; i < n * n; i++) {
    if (marks[i] !== SHADED) continue;
    const x = i % n, y = (i / n) | 0;
    for (const [dx, dy] of [[1,0],[0,1]]) {
      const nx = x + dx, ny = y + dy;
      if (nx>=n||ny>=n) continue;
      const ni = ny * n + nx;
      if (marks[ni] !== SHADED) continue;
      if (owner[ni] !== owner[i]) { bad.add(i); bad.add(ni); }
    }
  }
  // Per-region SHADED count + adjacency.
  for (const reg of regions) {
    const shadedCells = reg.filter(c => marks[c] === SHADED);
    if (shadedCells.length > 2) {
      for (const c of shadedCells) bad.add(c);
      continue;
    }
    if (shadedCells.length === 2) {
      const [a, b] = shadedCells;
      const ax = a % n, ay = (a / n) | 0;
      const bx = b % n, by = (b / n) | 0;
      const adj = (Math.abs(ax - bx) + Math.abs(ay - by)) === 1;
      if (!adj) { bad.add(a); bad.add(b); }
    }
  }
  return bad;
}

function isSolved(n, regions, marks) {
  for (let i = 0; i < n * n; i++) if (marks[i] === UNKNOWN) return false;
  if (findViolations(n, regions, marks).size) return false;
  // Each region has exactly 2 SHADED forming a domino.
  for (const reg of regions) {
    const shadedCells = reg.filter(c => marks[c] === SHADED);
    if (shadedCells.length !== 2) return false;
    const [a, b] = shadedCells;
    const ax = a % n, ay = (a / n) | 0;
    const bx = b % n, by = (b / n) | 0;
    if ((Math.abs(ax - bx) + Math.abs(ay - by)) !== 1) return false;
  }
  return true;
}
