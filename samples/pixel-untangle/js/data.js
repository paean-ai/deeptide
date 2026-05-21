// Pixel Untangle - the classic Planarity / Untangle puzzle.
//
// A graph of pegs joined by threads starts as a tangled knot. Drag the
// pegs around until no two threads cross. Every puzzle is generated from
// a known crossing-free layout, so a solution is guaranteed to exist.
//
// Each level is { n, seed }. buildPuzzle is deterministic in the seed and
// returns { n, edges, start, solved, levelIndex, cfg }.

const VW = 360, VH = 480;

// Pegs live inside this rectangle.
const AREA = { x0: 46, x1: 314, y0: 96, y1: 432 };

const LEVELS = [
  { name: ['Knot',    '小结'], n: 6,  seed: 211 },
  { name: ['Tangle',  '缠绕'], n: 8,  seed: 323 },
  { name: ['Lattice', '网格'], n: 10, seed: 437 },
  { name: ['Web',     '蛛网'], n: 12, seed: 541 },
  { name: ['Snarl',   '乱麻'], n: 14, seed: 659 },
  { name: ['Gordian', '死结'], n: 16, seed: 773 },
];
const LEVEL_COUNT = LEVELS.length;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () { s = (s * 16807) % 2147483647; return (s - 1) / 2147483646; };
}

// ---- segment geometry --------------------------------------------------
function ccw(a, b, c) { return (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x); }

// Do segments a-b and c-d properly cross? (Touching only at an endpoint
// does not count - those are handled by the shared-vertex check.)
function segCross(a, b, c, d) {
  const d1 = ccw(c, d, a), d2 = ccw(c, d, b);
  const d3 = ccw(a, b, c), d4 = ccw(a, b, d);
  return ((d1 > 0) !== (d2 > 0)) && ((d3 > 0) !== (d4 > 0));
}

function edgesAdjacent(e, f) {
  return e[0] === f[0] || e[0] === f[1] || e[1] === f[0] || e[1] === f[1];
}

// All crossings for the current peg positions. Returns the count plus the
// set of edge indices taking part in any crossing (for the red highlight).
function crossings(pos, edges) {
  const edgeSet = new Set();
  let count = 0;
  for (let i = 0; i < edges.length; i++) {
    for (let j = i + 1; j < edges.length; j++) {
      if (edgesAdjacent(edges[i], edges[j])) continue;
      if (segCross(pos[edges[i][0]], pos[edges[i][1]],
                   pos[edges[j][0]], pos[edges[j][1]])) {
        count++;
        edgeSet.add(i); edgeSet.add(j);
      }
    }
  }
  return { count, edgeSet };
}

function isSolved(pos, edges) { return crossings(pos, edges).count === 0; }

// ---- generation --------------------------------------------------------
// Scatter n pegs with a minimum separation; shrink the spacing if a layout
// cannot be found so generation always succeeds.
function placePegs(n, rng) {
  const w = AREA.x1 - AREA.x0, h = AREA.y1 - AREA.y0;
  let minD = Math.min(95, Math.sqrt(w * h / n) * 0.78);
  for (let pass = 0; pass < 40; pass++) {
    const pts = [];
    let tries = 0, ok = true;
    while (pts.length < n) {
      const p = { x: AREA.x0 + rng() * w, y: AREA.y0 + rng() * h };
      if (pts.every(q => Math.hypot(p.x - q.x, p.y - q.y) >= minD)) pts.push(p);
      if (++tries > 6000) { ok = false; break; }
    }
    if (ok) return pts;
    minD *= 0.85;
  }
  return null;
}

function buildPuzzle(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const n = cfg.n;
  const rng = seededRandom(cfg.seed);
  // A crossing-free reference layout - this is a guaranteed solution.
  const solved = placePegs(n, rng);
  // Add the shortest threads that do not cross one already placed.
  const cand = [];
  for (let i = 0; i < n; i++) {
    for (let j = i + 1; j < n; j++) {
      cand.push([i, j, Math.hypot(solved[i].x - solved[j].x, solved[i].y - solved[j].y)]);
    }
  }
  cand.sort((a, b) => a[2] - b[2]);
  const edges = [];
  const target = Math.round(n * 2.0);
  function fits(i, j) {
    for (const e of edges) {
      if (edgesAdjacent([i, j], e)) continue;
      if (segCross(solved[i], solved[j], solved[e[0]], solved[e[1]])) return false;
    }
    return true;
  }
  for (const [i, j] of cand) {
    if (edges.length >= target) break;
    if (fits(i, j)) edges.push([i, j]);
  }
  // No dangling pegs: give every peg at least two threads.
  const deg = new Array(n).fill(0);
  for (const e of edges) { deg[e[0]]++; deg[e[1]]++; }
  for (let i = 0; i < n; i++) {
    while (deg[i] < 2) {
      let bestJ = -1, bestD = 1e9;
      for (let j = 0; j < n; j++) {
        if (j === i) continue;
        if (edges.some(e => (e[0] === i && e[1] === j) || (e[0] === j && e[1] === i))) continue;
        if (!fits(i, j)) continue;
        const d = Math.hypot(solved[i].x - solved[j].x, solved[i].y - solved[j].y);
        if (d < bestD) { bestD = d; bestJ = j; }
      }
      if (bestJ < 0) break;
      edges.push([i, bestJ]); deg[i]++; deg[bestJ]++;
    }
  }
  // Scramble the pegs into a genuinely tangled start.
  let start = null;
  for (let t = 0; t < 200; t++) {
    const s = placePegs(n, rng);
    if (crossings(s, edges).count >= Math.max(2, Math.floor(n / 3))) { start = s; break; }
    start = s;
  }
  return { n, edges, start, solved, levelIndex, cfg };
}
