// Pixel Circuit - a rotate-the-wires network puzzle. Every tile is a wire
// piece; rotate them so power flows from the cell to every node with no loose
// end. The board is grown as a spanning tree, so a solution always exists.

const VW = 360, VH = 480;

// connection bits: N=1, E=2, S=4, W=8
// [dr, dc, bit toward neighbour, bit on the neighbour facing back]
const DIRS = [[-1, 0, 1, 4], [0, 1, 2, 8], [1, 0, 4, 1], [0, -1, 8, 2]];

const LEVELS = [
  { name: ['Spark', '火花'],   seed: 17,  n: 4 },
  { name: ['Relay', '中继'],   seed: 58,  n: 5 },
  { name: ['Array', '阵列'],   seed: 124, n: 5 },
  { name: ['Lattice', '网格'], seed: 211, n: 6 },
  { name: ['Mainframe', '主机'], seed: 333, n: 6 },
  { name: ['Reactor', '反应堆'], seed: 470, n: 7 },
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

// rotate a 4-bit direction mask 90 degrees clockwise, `times` times
function rotateMask(mask, times) {
  let m = mask;
  for (let k = 0; k < (times & 3); k++) m = ((m << 1) | (m >> 3)) & 15;
  return m;
}
function effMask(cell) { return rotateMask(cell.base, cell.rot); }

// grow a spanning tree of the grid (randomised DFS); base[i] = its wire mask
function growTree(n, rng) {
  const N = n * n;
  const base = new Array(N).fill(0);
  const seen = new Array(N).fill(false);
  const start = (rng() * N) | 0;
  const stack = [start];
  seen[start] = true;
  while (stack.length) {
    const cur = stack[stack.length - 1];
    const r = (cur / n) | 0, c = cur % n;
    const open = [];
    for (const [dr, dc, bit, obit] of DIRS) {
      const nr = r + dr, nc = c + dc;
      if (nr < 0 || nc < 0 || nr >= n || nc >= n) continue;
      const ni = nr * n + nc;
      if (!seen[ni]) open.push([ni, bit, obit]);
    }
    if (!open.length) { stack.pop(); continue; }
    const [ni, bit, obit] = open[(rng() * open.length) | 0];
    base[cur] |= bit;
    base[ni] |= obit;
    seen[ni] = true;
    stack.push(ni);
  }
  return base;
}

// rotations that leave a piece looking identical to its solved orientation
function solvedRots(mask) {
  const out = [];
  for (let t = 0; t < 4; t++) if (rotateMask(mask, t) === mask) out.push(t);
  return out;
}

// Build a level: grow the tree, scramble rotations, compute the par.
function buildPuzzle(level) {
  const n = level.n;
  const rng = seededRandom(level.seed);
  const base = growTree(n, rng);
  const source = ((n >> 1) * n) + (n >> 1);
  let cells, par;
  for (let attempt = 0; attempt < 60; attempt++) {
    cells = base.map(b => ({ base: b, rot: (rng() * 4) | 0 }));
    par = 0;
    for (const cell of cells) {
      const sr = solvedRots(cell.base);
      let best = 4;
      for (const t of sr) best = Math.min(best, ((t - cell.rot) % 4 + 4) % 4);
      par += best;
    }
    if (par > 0) break;
  }
  return { n, cells, source, par };
}

function rotateCell(pz, i) {
  pz.cells[i].rot = (pz.cells[i].rot + 1) & 3;
}

// Evaluate the board: which tiles are powered, whether any end leaks, win.
function evaluate(pz) {
  const n = pz.n, N = n * n;
  const eff = pz.cells.map(effMask);
  const adj = pz.cells.map(() => []);
  let leak = false;
  for (let i = 0; i < N; i++) {
    const r = (i / n) | 0, c = i % n;
    for (const [dr, dc, bit, obit] of DIRS) {
      if (!(eff[i] & bit)) continue;
      const nr = r + dr, nc = c + dc;
      if (nr < 0 || nc < 0 || nr >= n || nc >= n) { leak = true; continue; }
      const ni = nr * n + nc;
      if (eff[ni] & obit) adj[i].push(ni);
      else leak = true;
    }
  }
  const powered = new Array(N).fill(false);
  const stack = [pz.source];
  powered[pz.source] = true;
  while (stack.length) {
    const cur = stack.pop();
    for (const ni of adj[cur]) if (!powered[ni]) { powered[ni] = true; stack.push(ni); }
  }
  const allPowered = powered.every(Boolean);
  return { eff, powered, leak, won: !leak && allPowered };
}
