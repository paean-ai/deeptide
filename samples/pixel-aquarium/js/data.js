// Pixel Aquarium - hidden-water-level puzzle.
//
// Rules:
//   * The grid is partitioned into "tanks" (irregular regions).
//   * Each tank holds water at some single horizontal level. Cells of the
//     tank at or BELOW the level are filled (water); cells strictly above
//     are empty (air). Different tanks may have wildly different levels.
//   * Row / column numbers count the filled cells in each row / column.
//   * Deduce the unique combination of water levels.
//
// Each level is { n, k, seed }. buildPuzzle is deterministic in the seed
// and returns { n, regions, levels, fill, rc, cc, levelIndex }.

const VW = 360, VH = 480;

const LEVELS = [
  { name: ['Bay',     '小湾'], n: 5, k: 5, seed: 131 },
  { name: ['Cove',    '海湾'], n: 5, k: 5, seed: 287 },
  { name: ['Reef',    '礁石'], n: 6, k: 6, seed: 431 },
  { name: ['Lagoon',  '潟湖'], n: 6, k: 7, seed: 531 },
  { name: ['Abyss',   '深渊'], n: 7, k: 7, seed: 631 },
  { name: ['Trench',  '海沟'], n: 7, k: 8, seed: 733 },
];
const LEVEL_COUNT = LEVELS.length;

// Live cell tags:
const UNKNOWN = 0;
const AIR     = 1;
const WATER   = 2;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () {
    s = (s * 16807) % 2147483647;
    return (s - 1) / 2147483646;
  };
}

// ---- random tank partition ---------------------------------------------
// Round-robin BFS-expansion from k random seed cells. Reject if any region
// ends up smaller than 2 cells (boring + degenerate).
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

// ---- water-level math --------------------------------------------------
// waterLevel[r] is the smallest y in region r that is filled. Cells of r at
// y >= waterLevel[r] are water, cells at y < waterLevel[r] are air.
function applyWater(n, regions, levels) {
  const fill = new Array(n * n).fill(0);
  for (let r = 0; r < regions.length; r++) {
    const lv = levels[r];
    for (const c of regions[r]) {
      const y = (c / n) | 0;
      if (y >= lv) fill[c] = 1;
    }
  }
  return fill;
}

function rowColCounts(fill, n) {
  const rc = new Array(n).fill(0), cc = new Array(n).fill(0);
  for (let i = 0; i < n * n; i++) {
    if (!fill[i]) continue;
    const x = i % n, y = (i / n) | 0;
    rc[y]++; cc[x]++;
  }
  return { rc, cc };
}

// For each region, enumerate the possible water levels. Level y_min = full,
// y_max + 1 = empty. The first level above y_max represents the air-only
// state.
function regionLevels(regions, n) {
  return regions.map(reg => {
    let yMin = n, yMax = -1;
    for (const c of reg) { const y = (c / n) | 0; if (y < yMin) yMin = y; if (y > yMax) yMax = y; }
    const levels = [];
    for (let lv = yMin; lv <= yMax + 1; lv++) levels.push(lv);
    return levels;
  });
}

// ---- uniqueness solver -------------------------------------------------
// Backtrack region by region, trying each candidate water level. Prune by
// the row / col cap. At the leaf, confirm the row / col counts are exact.
function solveCount(n, regions, rc, cc, limit) {
  const levels = regionLevels(regions, n);
  const fill = new Array(n * n).fill(0);
  const rowS = new Array(n).fill(0);
  const colS = new Array(n).fill(0);
  let count = 0;
  function apply(idx, lv, val) {
    for (const c of regions[idx]) {
      const y = (c / n) | 0, x = c % n;
      const isFilled = (y >= lv);
      if (!isFilled) continue;
      fill[c] = val ? 1 : 0;
      if (val) { rowS[y]++; colS[x]++; }
      else     { rowS[y]--; colS[x]--; }
    }
  }
  function rec(idx) {
    if (count >= limit) return;
    if (idx === regions.length) {
      for (let i = 0; i < n; i++) if (rowS[i] !== rc[i] || colS[i] !== cc[i]) return;
      count++; return;
    }
    for (const lv of levels[idx]) {
      apply(idx, lv, true);
      let ok = true;
      for (let i = 0; i < n; i++) if (rowS[i] > rc[i] || colS[i] > cc[i]) { ok = false; break; }
      if (ok) rec(idx + 1);
      apply(idx, lv, false);
      if (count >= limit) return;
    }
  }
  rec(0);
  return count;
}

function buildPuzzle(levelIndex) {
  const cfg = LEVELS[levelIndex];
  for (let attempt = 0; attempt < 200; attempt++) {
    const rng = seededRandom(cfg.seed + attempt * 1009);
    const p = partition(cfg.n, cfg.k, rng);
    if (!p) continue;
    const levels = regionLevels(p.regions, cfg.n);
    const chosen = levels.map(lvs => lvs[(rng() * lvs.length) | 0]);
    const fill = applyWater(cfg.n, p.regions, chosen);
    const { rc, cc } = rowColCounts(fill, cfg.n);
    if (solveCount(cfg.n, p.regions, rc, cc, 2) === 1) {
      return { n: cfg.n, regions: p.regions, levels: chosen, fill, rc, cc, levelIndex, cfg };
    }
  }
  return null;
}

// ---- live validation ---------------------------------------------------
// `marks` is the live grid (UNKNOWN / AIR / WATER per cell).
// Returns the set of cells currently in conflict:
//   * row / col over-fill (too many WATER cells in a row or column).
//   * gravity violation: within a region, a WATER cell with an AIR cell
//     below it in the SAME region (water should fill bottom-up).
function findViolations(n, regions, rc, cc, marks) {
  const bad = new Set();
  const total = n * n;
  const owner = new Array(total).fill(-1);
  regions.forEach((r, id) => r.forEach(c => owner[c] = id));
  const rowS = new Array(n).fill(0);
  const colS = new Array(n).fill(0);
  for (let i = 0; i < total; i++) {
    if (marks[i] !== WATER) continue;
    const x = i % n, y = (i / n) | 0;
    rowS[y]++; colS[x]++;
  }
  for (let y = 0; y < n; y++) if (rowS[y] > rc[y]) {
    for (let x = 0; x < n; x++) if (marks[y * n + x] === WATER) bad.add(y * n + x);
  }
  for (let x = 0; x < n; x++) if (colS[x] > cc[x]) {
    for (let y = 0; y < n; y++) if (marks[y * n + x] === WATER) bad.add(y * n + x);
  }
  // Gravity: for each WATER cell, every region-mate at a STRICTLY greater y
  // (further down) marked AIR is a violation. The reverse (AIR above WATER
  // is fine).
  for (let i = 0; i < total; i++) {
    if (marks[i] !== WATER) continue;
    const x = i % n, y = (i / n) | 0;
    const reg = owner[i];
    for (const c of regions[reg]) {
      if (c === i) continue;
      const cx = c % n, cy = (c / n) | 0;
      if (cy <= y) continue;
      if (marks[c] === AIR) { bad.add(i); bad.add(c); }
    }
  }
  return bad;
}

function isSolved(n, regions, rc, cc, marks) {
  // Every cell decided.
  for (let i = 0; i < n * n; i++) if (marks[i] === UNKNOWN) return false;
  if (findViolations(n, regions, rc, cc, marks).size) return false;
  // Row / col counts exact.
  const rowS = new Array(n).fill(0);
  const colS = new Array(n).fill(0);
  for (let i = 0; i < n * n; i++) {
    if (marks[i] !== WATER) continue;
    const x = i % n, y = (i / n) | 0;
    rowS[y]++; colS[x]++;
  }
  for (let i = 0; i < n; i++) if (rowS[i] !== rc[i] || colS[i] !== cc[i]) return false;
  return true;
}
