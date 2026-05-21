// Pixel Magnets - the magnet-shading logic puzzle.
//
// Rules:
//   * The grid is pre-tiled with 1×2 / 2×1 dominoes (the "magnets").
//   * Each magnet is either NEUTRAL or carries one '+' on one end and one
//     '-' on the other. So the two cells of a magnet either both stay
//     empty, or one of them is '+' and the other is '-'.
//   * No two cells with the same charge may share an edge (across magnets).
//   * Row / column numbers count the '+' cells on the left and '-' cells
//     on the right (and similarly for columns at top / bottom).
//   * Find the unique state of every magnet.
//
// buildPuzzle is deterministic per seed and returns
//   { W, H, dominoes, solutionStates, rc, levelIndex, cfg }.

const VW = 360, VH = 480;

const LEVELS = [
  { name: ['Spark',    '火花'], W: 4, H: 4, seed: 131 },
  { name: ['Coil',     '线圈'], W: 4, H: 4, seed: 287 },
  { name: ['Loadstone','磁石'], W: 4, H: 6, seed: 431 },
  { name: ['Solenoid', '螺线'], W: 6, H: 4, seed: 531 },
  { name: ['Reactor',  '反应堆'], W: 6, H: 6, seed: 631 },
  { name: ['Tokamak',  '托卡马克'], W: 6, H: 6, seed: 733 },
  { name: ['Dynamo',   '发电机'], W: 6, H: 8, seed: 805 },
  { name: ['Pulsar',   '脉冲星'], W: 8, H: 6, seed: 913 },
  { name: ['Magnetar', '磁星'],   W: 8, H: 8, seed: 1029 },
];
const LEVEL_COUNT = LEVELS.length;

// State tags for the live grid:
const UNKNOWN = 0;
const POS     = 1;       // '+'
const NEG     = 2;       // '-'
const NEUTRAL = 3;       // explicitly marked empty (X)

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () {
    s = (s * 16807) % 2147483647;
    return (s - 1) / 2147483646;
  };
}

// ---- random domino tiling ----------------------------------------------
function randomTiling(W, H, rng) {
  for (let attempt = 0; attempt < 200; attempt++) {
    const grid = new Array(W * H).fill(-1);
    let id = 0;
    let ok = true;
    for (let y = 0; y < H; y++) {
      for (let x = 0; x < W; x++) {
        if (grid[y * W + x] !== -1) continue;
        const canH = x + 1 < W && grid[y * W + x + 1] === -1;
        const canV = y + 1 < H && grid[(y + 1) * W + x] === -1;
        if (!canH && !canV) { ok = false; break; }
        let h = canH;
        if (canH && canV) h = rng() < 0.5;
        if (h) { grid[y * W + x] = id; grid[y * W + x + 1] = id; }
        else   { grid[y * W + x] = id; grid[(y + 1) * W + x] = id; }
        id++;
      }
      if (!ok) break;
    }
    if (!ok) continue;
    const dominoes = [];
    for (let i = 0; i < id; i++) dominoes.push([]);
    for (let i = 0; i < W * H; i++) dominoes[grid[i]].push(i);
    return { grid, dominoes };
  }
  return null;
}

// state per domino: 0 = NEUTRAL, 1 = first cell + / second -, 2 = first - / second +.
// Cell order inside a domino is row-major: first cell is the upper-left of the pair.
function chargeFromStates(W, H, dominoes, states) {
  const charge = new Array(W * H).fill(0);
  for (let i = 0; i < dominoes.length; i++) {
    const st = states[i];
    if (st === 0) continue;
    const [a, b] = dominoes[i];
    if (st === 1) { charge[a] = 1;  charge[b] = -1; }
    else          { charge[a] = -1; charge[b] = 1;  }
  }
  return charge;
}

function noAdjConflict(W, H, dominoes, states) {
  const charge = chargeFromStates(W, H, dominoes, states);
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
    const i = y * W + x;
    if (!charge[i]) continue;
    for (const [dx, dy] of [[1, 0], [0, 1]]) {
      const nx = x + dx, ny = y + dy;
      if (nx >= W || ny >= H) continue;
      const ni = ny * W + nx;
      if (charge[ni] === charge[i]) return false;
    }
  }
  return true;
}

function rowColCounts(W, H, dominoes, states) {
  const charge = chargeFromStates(W, H, dominoes, states);
  const rowP = new Array(H).fill(0), rowN = new Array(H).fill(0);
  const colP = new Array(W).fill(0), colN = new Array(W).fill(0);
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
    const c = charge[y * W + x];
    if (c > 0) { rowP[y]++; colP[x]++; }
    else if (c < 0) { rowN[y]++; colN[x]++; }
  }
  return { rowP, rowN, colP, colN };
}

// ---- uniqueness solver -------------------------------------------------
// Backtrack over dominoes, try each of the 3 states; prune by neighbour
// adjacency (no same-charge edge-touching) and by row / col cap.
function solveCount(W, H, dominoes, rc, limit) {
  const charge = new Array(W * H).fill(0);
  const rowP = new Array(H).fill(0), rowN = new Array(H).fill(0);
  const colP = new Array(W).fill(0), colN = new Array(W).fill(0);
  let count = 0;
  function applyState(idx, st, sign) {
    const [a, b] = dominoes[idx];
    const ax = a % W, ay = (a / W) | 0, bx = b % W, by = (b / W) | 0;
    let ca = 0, cb = 0;
    if (st === 1) { ca = 1; cb = -1; }
    else if (st === 2) { ca = -1; cb = 1; }
    if (sign > 0) { charge[a] = ca; charge[b] = cb; }
    else { charge[a] = 0; charge[b] = 0; }
    if (sign > 0) {
      if (ca > 0) { rowP[ay]++; colP[ax]++; } else if (ca < 0) { rowN[ay]++; colN[ax]++; }
      if (cb > 0) { rowP[by]++; colP[bx]++; } else if (cb < 0) { rowN[by]++; colN[bx]++; }
    } else {
      if (ca > 0) { rowP[ay]--; colP[ax]--; } else if (ca < 0) { rowN[ay]--; colN[ax]--; }
      if (cb > 0) { rowP[by]--; colP[bx]--; } else if (cb < 0) { rowN[by]--; colN[bx]--; }
    }
  }
  function adjOkAt(c) {
    const x = c % W, y = (c / W) | 0;
    for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) {
      const nx = x + dx, ny = y + dy;
      if (nx < 0 || nx >= W || ny < 0 || ny >= H) continue;
      const ni = ny * W + nx;
      if (charge[c] !== 0 && charge[ni] === charge[c]) return false;
    }
    return true;
  }
  function rec(idx) {
    if (count >= limit) return;
    if (idx === dominoes.length) {
      for (let i = 0; i < H; i++) {
        if (rowP[i] !== rc.rowP[i] || rowN[i] !== rc.rowN[i]) return;
      }
      for (let i = 0; i < W; i++) {
        if (colP[i] !== rc.colP[i] || colN[i] !== rc.colN[i]) return;
      }
      count++;
      return;
    }
    for (let st = 0; st < 3; st++) {
      applyState(idx, st, 1);
      const [a, b] = dominoes[idx];
      let ok = adjOkAt(a) && adjOkAt(b);
      const ay = (a / W) | 0, ax = a % W, by = (b / W) | 0, bx = b % W;
      if (rowP[ay] > rc.rowP[ay] || rowN[ay] > rc.rowN[ay]
       || colP[ax] > rc.colP[ax] || colN[ax] > rc.colN[ax]) ok = false;
      if (rowP[by] > rc.rowP[by] || rowN[by] > rc.rowN[by]
       || colP[bx] > rc.colP[bx] || colN[bx] > rc.colN[bx]) ok = false;
      if (ok) rec(idx + 1);
      applyState(idx, st, -1);
      if (count >= limit) return;
    }
  }
  rec(0);
  return count;
}

// Pick a random valid magnet assignment + verify uniqueness from row/col
// counts. Hint cells are not used — the row/col charge counts are the
// only clues.
function buildPuzzle(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const { W, H, seed } = cfg;
  for (let attempt = 0; attempt < 80; attempt++) {
    const rng = seededRandom(seed + attempt * 1009);
    const til = randomTiling(W, H, rng);
    if (!til) continue;
    const { dominoes } = til;
    const states = new Array(dominoes.length);
    let valid = null;
    for (let k = 0; k < 30; k++) {
      for (let i = 0; i < dominoes.length; i++) states[i] = (rng() * 3) | 0;
      if (noAdjConflict(W, H, dominoes, states)) { valid = states.slice(); break; }
    }
    if (!valid) continue;
    const rc = rowColCounts(W, H, dominoes, valid);
    if (solveCount(W, H, dominoes, rc, 2) === 1) {
      return { W, H, dominoes, solutionStates: valid, rc, levelIndex, cfg };
    }
  }
  return null;
}

// ---- live validation ---------------------------------------------------
// `marks` is the live grid: UNKNOWN / POS / NEG / NEUTRAL per cell.
// Domino consistency: both cells must agree on what magnet state the player
// thinks it's in (NEUTRAL/+-/-+); a mismatch flags both cells.
// Adjacency: two same-charge cells edge-touching flags both.
// Row/col over-count flags every same-charge cell in that line.
function findViolations(W, H, dominoes, rc, marks) {
  const bad = new Set();
  // Domino consistency.
  const cellDom = new Array(W * H);
  dominoes.forEach((d, id) => d.forEach(c => cellDom[c] = id));
  for (const [a, b] of dominoes) {
    const ma = marks[a], mb = marks[b];
    const okPair =
      (ma === UNKNOWN || mb === UNKNOWN) ||
      (ma === NEUTRAL && mb === NEUTRAL) ||
      (ma === POS && mb === NEG) ||
      (ma === NEG && mb === POS);
    if (!okPair) { bad.add(a); bad.add(b); }
  }
  // Adjacency.
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
    const i = y * W + x;
    if (marks[i] !== POS && marks[i] !== NEG) continue;
    for (const [dx, dy] of [[1, 0], [0, 1]]) {
      const nx = x + dx, ny = y + dy;
      if (nx >= W || ny >= H) continue;
      const ni = ny * W + nx;
      if (marks[ni] === marks[i]) { bad.add(i); bad.add(ni); }
    }
  }
  // Row / col over-counts.
  for (let y = 0; y < H; y++) {
    let p = 0, n = 0;
    for (let x = 0; x < W; x++) {
      const m = marks[y * W + x];
      if (m === POS) p++;
      else if (m === NEG) n++;
    }
    if (p > rc.rowP[y] || n > rc.rowN[y]) {
      for (let x = 0; x < W; x++) {
        const m = marks[y * W + x];
        if (m === POS || m === NEG) bad.add(y * W + x);
      }
    }
  }
  for (let x = 0; x < W; x++) {
    let p = 0, n = 0;
    for (let y = 0; y < H; y++) {
      const m = marks[y * W + x];
      if (m === POS) p++;
      else if (m === NEG) n++;
    }
    if (p > rc.colP[x] || n > rc.colN[x]) {
      for (let y = 0; y < H; y++) {
        const m = marks[y * W + x];
        if (m === POS || m === NEG) bad.add(y * W + x);
      }
    }
  }
  return bad;
}

function isSolved(W, H, dominoes, rc, marks) {
  for (let i = 0; i < W * H; i++) if (marks[i] === UNKNOWN) return false;
  if (findViolations(W, H, dominoes, rc, marks).size) return false;
  // Exact counts.
  for (let y = 0; y < H; y++) {
    let p = 0, n = 0;
    for (let x = 0; x < W; x++) {
      const m = marks[y * W + x];
      if (m === POS) p++;
      else if (m === NEG) n++;
    }
    if (p !== rc.rowP[y] || n !== rc.rowN[y]) return false;
  }
  for (let x = 0; x < W; x++) {
    let p = 0, n = 0;
    for (let y = 0; y < H; y++) {
      const m = marks[y * W + x];
      if (m === POS) p++;
      else if (m === NEG) n++;
    }
    if (p !== rc.colP[x] || n !== rc.colN[x]) return false;
  }
  return true;
}
