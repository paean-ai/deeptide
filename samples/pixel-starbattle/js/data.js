// Pixel Star Battle - region-partition + star placement logic puzzle.
//
// Rules:
//   * The N×N grid is divided into N irregular regions.
//   * Place exactly K stars in every row, every column, and every region.
//   * Stars cannot touch each other - not even diagonally.
//
// Each level is { n, k, seed }. buildPuzzle is deterministic in the seed and
// returns { regions, solution } where solution[i] = 1 if cell i carries a star.

const VW = 360, VH = 480;

const LEVELS = [
  { name: ['Sky',     '夜空'],  n: 5, k: 1, seed: 131  },
  { name: ['Cosmos',  '寰宇'],  n: 5, k: 1, seed: 281  },
  { name: ['Nebula',  '星云'],  n: 6, k: 1, seed: 602  },
  { name: ['Galaxy',  '星系'],  n: 6, k: 1, seed: 813  },
  { name: ['Cluster', '星团'],  n: 7, k: 1, seed: 1413 },
  { name: ['Quasar',  '类星'],  n: 8, k: 1, seed: 1714 },
  { name: ['Pulsar',  '脉冲星'], n: 8,  k: 2, seed: 201 },
  { name: ['Supernova', '超新星'], n: 9, k: 2, seed: 401 },
  { name: ['Singularity', '奇点'], n: 10, k: 2, seed: 622 },
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

// Build n random regions by round-robin BFS-expansion from n distinct seeds.
function partition(n, rng) {
  const total = n * n;
  const seeds = new Set();
  while (seeds.size < n) seeds.add((rng() * total) | 0);
  const owner = new Array(total).fill(-1);
  const fronts = [];
  let id = 0;
  for (const s of seeds) { owner[s] = id; fronts.push([s]); id++; }
  let remaining = total - n;
  while (remaining > 0) {
    let progressed = false;
    for (let r = 0; r < n; r++) {
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
  const regions = Array.from({ length: n }, () => []);
  for (let i = 0; i < total; i++) regions[owner[i]].push(i);
  for (const r of regions) if (r.length < 2) return null;
  return regions;
}

// Enumerate star placements row-by-row up to `limit` solutions.
// `acceptedAt` is filled with the cells of the final solution.
function solve(n, regions, k, limit, acceptedAt) {
  const total = n * n;
  const owner = new Array(total);
  regions.forEach((r, id) => r.forEach(c => owner[c] = id));
  const colS = new Array(n).fill(0);
  const regS = new Array(n).fill(0);
  const grid = new Array(total).fill(0);
  let count = 0;
  let saved = null;
  function placeInRow(row, startCol, placedInRow) {
    if (placedInRow === k) {
      if (row === n - 1) {
        count++;
        if (count === 1 && acceptedAt) {
          saved = [];
          for (let i = 0; i < total; i++) if (grid[i]) saved.push(i);
        }
        return;
      }
      placeInRow(row + 1, 0, 0);
      return;
    }
    if (startCol >= n) return;
    for (let c = startCol; c < n; c++) {
      if (count >= limit) return;
      if (colS[c] >= k) continue;
      const reg = owner[row * n + c];
      if (regS[reg] >= k) continue;
      let ok = true;
      for (let dy = -1; dy <= 1 && ok; dy++) {
        for (let dx = -1; dx <= 1 && ok; dx++) {
          if (!dx && !dy) continue;
          const nx = c + dx, ny = row + dy;
          if (nx<0||nx>=n||ny<0||ny>=n) continue;
          if (grid[ny * n + nx]) ok = false;
        }
      }
      if (!ok) continue;
      grid[row * n + c] = 1;
      colS[c]++; regS[reg]++;
      placeInRow(row, c + 2, placedInRow + 1);
      grid[row * n + c] = 0;
      colS[c]--; regS[reg]--;
      if (count >= limit) return;
    }
  }
  placeInRow(0, 0, 0);
  if (acceptedAt && saved) {
    acceptedAt.length = 0;
    for (const c of saved) acceptedAt.push(c);
  }
  return count;
}

function solveCount(n, regions, k, limit) {
  return solve(n, regions, k, limit, null);
}

function buildPuzzle(levelIndex) {
  const cfg = LEVELS[levelIndex];
  for (let attempt = 0; attempt < 2000; attempt++) {
    const rng = seededRandom(cfg.seed + attempt * 1009);
    const regs = partition(cfg.n, rng);
    if (!regs) continue;
    const acc = [];
    const ct = solve(cfg.n, regs, cfg.k, 2, acc);
    if (ct === 1) {
      const solution = new Array(cfg.n * cfg.n).fill(0);
      for (const c of acc) solution[c] = 1;
      return { n: cfg.n, k: cfg.k, regions: regs, solution, levelIndex };
    }
  }
  return null;
}

// ---- live validation (for UI red-flag) ---------------------------------
// `marks` is the live grid where each cell is 0 (blank), 1 (star), or 2 (X).
// We highlight any cell whose STAR is currently in conflict with the rules.
function findViolations(n, k, regions, marks) {
  const bad = new Set();
  const total = n * n;
  const owner = new Array(total);
  regions.forEach((r, id) => r.forEach(c => owner[c] = id));
  // Per-row, per-col, per-region star count > k.
  const rowS = new Array(n).fill(0);
  const colS = new Array(n).fill(0);
  const regS = new Array(n).fill(0);
  const rowCells = Array.from({ length: n }, () => []);
  const colCells = Array.from({ length: n }, () => []);
  const regCells = Array.from({ length: n }, () => []);
  for (let i = 0; i < total; i++) {
    if (marks[i] !== 1) continue;
    const x = i % n, y = (i / n) | 0;
    rowS[y]++; colS[x]++; regS[owner[i]]++;
    rowCells[y].push(i); colCells[x].push(i); regCells[owner[i]].push(i);
  }
  for (let y = 0; y < n; y++) if (rowS[y] > k) for (const c of rowCells[y]) bad.add(c);
  for (let x = 0; x < n; x++) if (colS[x] > k) for (const c of colCells[x]) bad.add(c);
  for (let r = 0; r < n; r++) if (regS[r] > k) for (const c of regCells[r]) bad.add(c);
  // Adjacent stars (including diagonal).
  for (let i = 0; i < total; i++) {
    if (marks[i] !== 1) continue;
    const x = i % n, y = (i / n) | 0;
    for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) {
      if (!dx && !dy) continue;
      const nx = x + dx, ny = y + dy;
      if (nx<0||nx>=n||ny<0||ny>=n) continue;
      const ni = ny * n + nx;
      if (marks[ni] === 1) { bad.add(i); bad.add(ni); }
    }
  }
  return bad;
}

function isSolved(n, k, regions, marks) {
  const total = n * n;
  let stars = 0;
  for (let i = 0; i < total; i++) if (marks[i] === 1) stars++;
  if (stars !== n * k) return false;
  if (findViolations(n, k, regions, marks).size) return false;
  // Verify row/col/region counts are exactly k.
  const owner = new Array(total);
  regions.forEach((r, id) => r.forEach(c => owner[c] = id));
  const rowS = new Array(n).fill(0);
  const colS = new Array(n).fill(0);
  const regS = new Array(n).fill(0);
  for (let i = 0; i < total; i++) {
    if (marks[i] !== 1) continue;
    const x = i % n, y = (i / n) | 0;
    rowS[y]++; colS[x]++; regS[owner[i]]++;
  }
  for (let i = 0; i < n; i++) if (rowS[i] !== k || colS[i] !== k || regS[i] !== k) return false;
  return true;
}
