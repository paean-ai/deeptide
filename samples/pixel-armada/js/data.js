// Pixel Armada - Battleship Solitaire (Bimaru).
//
// Rules:
//   * Place the level's fleet so every ship sits straight (horiz or vert) and
//     no two ships touch each other - not even diagonally.
//   * The row / column numbers count ship cells in that row / column.
//   * Some cells start revealed as either ship or water hints.
//   * Find the unique placement.
//
// buildPuzzle is deterministic per seed and returns
//   { n, fleet, rc, cc, hints, solution }
// where solution[i] = 1 if cell i is a ship cell.

const VW = 360, VH = 480;

const LEVELS = [
  { name: ['Skiff',     '小艇'], n: 5, fleet: [3, 2, 2, 1, 1],                   seed: 131 },
  { name: ['Dinghy',    '舢板'], n: 5, fleet: [3, 2, 2, 1, 1, 1],                seed: 231 },
  { name: ['Cutter',    '快艇'], n: 6, fleet: [3, 2, 2, 2, 1, 1, 1],             seed: 331 },
  { name: ['Frigate',   '护卫'], n: 6, fleet: [4, 3, 2, 2, 1, 1, 1],             seed: 431 },
  { name: ['Cruiser',   '巡洋'], n: 7, fleet: [4, 3, 3, 2, 2, 1, 1],             seed: 531 },
  { name: ['Armada',    '舰队'], n: 7, fleet: [4, 3, 3, 2, 2, 2, 1, 1, 1, 1],    seed: 631 },
];
const LEVEL_COUNT = LEVELS.length;

// Cell tag for live grid:
const UNKNOWN = -1;
const WATER   = 0;
const SHIP    = 1;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () {
    s = (s * 16807) % 2147483647;
    return (s - 1) / 2147483646;
  };
}

// ---- random fleet placement --------------------------------------------
// Place ships in descending size order at random valid (x, y, dir). Each
// ship may not touch another ship in any of the 8 neighbour cells. Reject
// the attempt if any ship can't be placed.
function placeFleet(n, fleet, rng) {
  for (let attempt = 0; attempt < 400; attempt++) {
    const occ = new Array(n * n).fill(0);
    let ok = true;
    for (const size of fleet) {
      const positions = [];
      for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) {
        if (x + size <= n)              positions.push([x, y, 0]);
        if (size > 1 && y + size <= n)  positions.push([x, y, 1]);
      }
      for (let i = positions.length - 1; i > 0; i--) {
        const j = (rng() * (i + 1)) | 0;
        [positions[i], positions[j]] = [positions[j], positions[i]];
      }
      let placed = false;
      for (const [x, y, d] of positions) {
        let valid = true;
        for (let k = 0; k < size; k++) {
          const cx = d === 0 ? x + k : x, cy = d === 0 ? y : y + k;
          if (occ[cy * n + cx]) { valid = false; break; }
          for (let dy = -1; dy <= 1 && valid; dy++) for (let dx = -1; dx <= 1 && valid; dx++) {
            const nx = cx + dx, ny = cy + dy;
            if (nx<0||nx>=n||ny<0||ny>=n) continue;
            if (occ[ny * n + nx]) valid = false;
          }
        }
        if (!valid) continue;
        for (let k = 0; k < size; k++) {
          const cx = d === 0 ? x + k : x, cy = d === 0 ? y : y + k;
          occ[cy * n + cx] = 1;
        }
        placed = true;
        break;
      }
      if (!placed) { ok = false; break; }
    }
    if (!ok) continue;
    return occ;
  }
  return null;
}

// ---- uniqueness solver --------------------------------------------------
// Count solutions of the puzzle (fleet, rc, cc, hints) up to `limit`. The
// solver tries each ship in turn at every valid position, pruning by:
//   * the row/col count cap.
//   * water hints (forbid ship cells there).
//   * the no-touching-other-ships rule.
//   * a lexicographic dedup for ships of equal size in the fleet list (so
//     the two cruisers don't double-count their swap).
// At the leaf it confirms row/col counts match exactly and every ship hint
// is now under a ship.
function solveCount(n, fleet, rc, cc, hints, limit) {
  const grid = new Array(n * n).fill(0);
  const shipHints = new Set();
  for (let i = 0; i < n * n; i++) if (hints[i] === 1) shipHints.add(i);
  const rowS = new Array(n).fill(0);
  const colS = new Array(n).fill(0);
  let count = 0;
  function canPlace(x, y, d, size) {
    for (let k = 0; k < size; k++) {
      const cx = d === 0 ? x + k : x, cy = d === 0 ? y : y + k;
      if (cx<0||cx>=n||cy<0||cy>=n) return false;
      if (hints[cy * n + cx] === 0) return false;
      if (grid[cy * n + cx]) return false;
      if (rowS[cy] + 1 > rc[cy]) return false;
      if (colS[cx] + 1 > cc[cx]) return false;
      for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) {
        const nx = cx + dx, ny = cy + dy;
        if (nx<0||nx>=n||ny<0||ny>=n) continue;
        let same = false;
        for (let k2 = 0; k2 < size; k2++) {
          const ax = d === 0 ? x + k2 : x, ay = d === 0 ? y : y + k2;
          if (ax === nx && ay === ny) { same = true; break; }
        }
        if (same) continue;
        if (grid[ny * n + nx]) return false;
      }
    }
    return true;
  }
  function place(x, y, d, size, val) {
    for (let k = 0; k < size; k++) {
      const cx = d === 0 ? x + k : x, cy = d === 0 ? y : y + k;
      grid[cy * n + cx] = val;
      if (val) { rowS[cy]++; colS[cx]++; }
      else     { rowS[cy]--; colS[cx]--; }
    }
  }
  function rec(shipIdx, prev) {
    if (count >= limit) return;
    if (shipIdx === fleet.length) {
      for (const hi of shipHints) if (!grid[hi]) return;
      for (let i = 0; i < n; i++) {
        if (rowS[i] !== rc[i]) return;
        if (colS[i] !== cc[i]) return;
      }
      count++;
      return;
    }
    const size = fleet[shipIdx];
    for (let d = 0; d <= 1; d++) {
      if (size === 1 && d === 1) continue;       // size-1 ship has no orientation
      const ymax = d === 0 ? n : n - size + 1;
      const xmax = d === 0 ? n - size + 1 : n;
      for (let y = 0; y < ymax; y++) for (let x = 0; x < xmax; x++) {
        const code = d * n * n + y * n + x;
        if (code <= prev) continue;               // dedup same-size ships
        if (!canPlace(x, y, d, size)) continue;
        place(x, y, d, size, 1);
        const nextPrev = (shipIdx + 1 < fleet.length && fleet[shipIdx + 1] === size) ? code : -1;
        rec(shipIdx + 1, nextPrev);
        place(x, y, d, size, 0);
        if (count >= limit) return;
      }
    }
  }
  rec(0, -1);
  return count;
}

function rowColCounts(occ, n) {
  const rc = new Array(n).fill(0), cc = new Array(n).fill(0);
  for (let i = 0; i < n * n; i++) {
    if (!occ[i]) continue;
    const x = i % n, y = (i / n) | 0;
    rc[y]++; cc[x]++;
  }
  return { rc, cc };
}

// Greedy hint reveal: start with no hints, then reveal ship cells (in random
// order) until the puzzle is uniquely solvable. Water hints aren't needed -
// the row/col counts already tell you where water sits.
function makePuzzle(n, fleet, rng) {
  const occ = placeFleet(n, fleet, rng);
  if (!occ) return null;
  const { rc, cc } = rowColCounts(occ, n);
  const hints = new Array(n * n).fill(-1);
  let ct = solveCount(n, fleet, rc, cc, hints, 2);
  if (ct === 1) return { n, fleet, rc, cc, hints, solution: occ };
  const shipCells = [];
  for (let i = 0; i < n * n; i++) if (occ[i]) shipCells.push(i);
  for (let i = shipCells.length - 1; i > 0; i--) {
    const j = (rng() * (i + 1)) | 0;
    [shipCells[i], shipCells[j]] = [shipCells[j], shipCells[i]];
  }
  for (const sc of shipCells) {
    hints[sc] = 1;
    ct = solveCount(n, fleet, rc, cc, hints, 2);
    if (ct === 1) return { n, fleet, rc, cc, hints, solution: occ };
    if (ct === 0) return null;
  }
  return null;
}

function buildPuzzle(levelIndex) {
  const cfg = LEVELS[levelIndex];
  for (let attempt = 0; attempt < 200; attempt++) {
    const rng = seededRandom(cfg.seed + attempt * 1009);
    const p = makePuzzle(cfg.n, cfg.fleet, rng);
    if (p) return { ...p, levelIndex, cfg };
  }
  return null;
}

// ---- live validation ---------------------------------------------------
// `marks` is the live grid: UNKNOWN / WATER / SHIP per cell.
// Returns the set of cells currently in conflict:
//   * any two SHIP cells touching diagonally that aren't the same straight ship
//     (i.e., two ships pinched at a corner).
//   * a row/col with more SHIP cells than its count.
function findViolations(n, rc, cc, marks) {
  const bad = new Set();
  const total = n * n;
  const rowS = new Array(n).fill(0);
  const colS = new Array(n).fill(0);
  for (let i = 0; i < total; i++) {
    if (marks[i] !== SHIP) continue;
    const x = i % n, y = (i / n) | 0;
    rowS[y]++; colS[x]++;
  }
  for (let y = 0; y < n; y++) if (rowS[y] > rc[y]) {
    for (let x = 0; x < n; x++) if (marks[y * n + x] === SHIP) bad.add(y * n + x);
  }
  for (let x = 0; x < n; x++) if (colS[x] > cc[x]) {
    for (let y = 0; y < n; y++) if (marks[y * n + x] === SHIP) bad.add(y * n + x);
  }
  // Diagonal-touch detection: any two ship cells that are diagonal neighbours
  // and not also horizontally/vertically adjacent (which would be the same
  // ship's body) flag.
  for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) {
    if (marks[y * n + x] !== SHIP) continue;
    for (const [dx, dy] of [[1, 1], [-1, 1], [1, -1], [-1, -1]]) {
      const nx = x + dx, ny = y + dy;
      if (nx<0||nx>=n||ny<0||ny>=n) continue;
      if (marks[ny * n + nx] !== SHIP) continue;
      // It's only a violation if NEITHER straight neighbour between them is
      // also a ship cell (which would mean an L-bend - illegal anyway).
      const horiz = marks[y * n + nx] === SHIP;
      const vert  = marks[ny * n + x] === SHIP;
      if (!horiz && !vert) {
        bad.add(y * n + x);
        bad.add(ny * n + nx);
      } else {
        // L-bend or T - also illegal in Battleship.
        bad.add(y * n + x);
        bad.add(ny * n + nx);
      }
    }
  }
  return bad;
}

function isSolved(n, rc, cc, fleet, marks) {
  if (findViolations(n, rc, cc, marks).size) return false;
  // Row/col counts exact.
  const rowS = new Array(n).fill(0);
  const colS = new Array(n).fill(0);
  for (let i = 0; i < n * n; i++) {
    if (marks[i] === SHIP) {
      const x = i % n, y = (i / n) | 0;
      rowS[y]++; colS[x]++;
    }
  }
  for (let i = 0; i < n; i++) if (rowS[i] !== rc[i] || colS[i] !== cc[i]) return false;
  // Connected components of ship cells, each must be a straight 1xK with K in fleet.
  const seen = new Array(n * n).fill(false);
  const found = [];
  for (let i = 0; i < n * n; i++) {
    if (marks[i] !== SHIP || seen[i]) continue;
    // BFS orthogonal.
    const comp = [];
    const stack = [i];
    seen[i] = true;
    while (stack.length) {
      const c = stack.pop();
      comp.push(c);
      const x = c % n, y = (c / n) | 0;
      for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) {
        const nx = x + dx, ny = y + dy;
        if (nx<0||nx>=n||ny<0||ny>=n) continue;
        const ni = ny * n + nx;
        if (marks[ni] === SHIP && !seen[ni]) { seen[ni] = true; stack.push(ni); }
      }
    }
    // Must be a straight line (all same row OR all same col).
    const xs = new Set(comp.map(c => c % n));
    const ys = new Set(comp.map(c => (c / n) | 0));
    if (xs.size !== 1 && ys.size !== 1) return false;
    found.push(comp.length);
  }
  // Multiset of ship sizes must match fleet.
  const a = found.slice().sort((x, y) => y - x);
  const b = fleet.slice().sort((x, y) => y - x);
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
  return true;
}
