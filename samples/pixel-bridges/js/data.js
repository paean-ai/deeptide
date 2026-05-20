// Pixel Bridges - Hashiwokakero puzzles: generation and the verifying solver.
//
// Each puzzle is a grid size + island count + seed. From the seed a connected
// network of islands and bridges is grown; the bridge counts become the island
// clues. A backtracking solver then confirms the clues have exactly ONE
// solution, so every level is a genuine, uniquely-solvable bridges puzzle.

const VW = 360, VH = 480;

const PUZZLES = [
  { name: ['Cove', '小湾'],     seed: 12,  w: 7, h: 7, count: 8 },
  { name: ['Harbor', '海港'],   seed: 47,  w: 7, h: 7, count: 9 },
  { name: ['Channel', '海峡'],  seed: 95,  w: 8, h: 8, count: 11 },
  { name: ['Archipelago', '群岛'], seed: 168, w: 8, h: 8, count: 12 },
  { name: ['Expanse', '汪洋'],  seed: 264, w: 9, h: 9, count: 13 },
  { name: ['Maelstrom', '漩涡'], seed: 390, w: 9, h: 9, count: 15 },
  { name: ['Tempest', '风暴'],   seed: 512, w: 9, h: 9, count: 16 },
  { name: ['Leviathan', '巨灵'], seed: 644, w: 10, h: 10, count: 17 },
  { name: ['Behemoth', '巨兽'],  seed: 781, w: 10, h: 10, count: 18 },
  { name: ['Kraken', '海怪'],    seed: 912, w: 10, h: 10, count: 19 },
];
const PUZZLE_COUNT = PUZZLES.length;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () {
    s = (s * 16807) % 2147483647;
    return (s - 1) / 2147483646;
  };
}

const DIRS = [[0, -1], [0, 1], [-1, 0], [1, 0]];

// All bridgeable pairs: consecutive islands sharing a row or column.
function candidateEdges(islands) {
  const edges = [];
  const byRow = {}, byCol = {};
  islands.forEach((is, i) => {
    (byRow[is.r] || (byRow[is.r] = [])).push(i);
    (byCol[is.c] || (byCol[is.c] = [])).push(i);
  });
  for (const k in byRow) {
    const line = byRow[k].sort((a, b) => islands[a].c - islands[b].c);
    for (let i = 0; i + 1 < line.length; i++) {
      const a = line[i], b = line[i + 1];
      edges.push({ a, b, horiz: true, ar: islands[a].r, ac: islands[a].c, br: islands[b].r, bc: islands[b].c });
    }
  }
  for (const k in byCol) {
    const line = byCol[k].sort((a, b) => islands[a].r - islands[b].r);
    for (let i = 0; i + 1 < line.length; i++) {
      const a = line[i], b = line[i + 1];
      edges.push({ a, b, horiz: false, ar: islands[a].r, ac: islands[a].c, br: islands[b].r, bc: islands[b].c });
    }
  }
  return edges;
}

function edgesCross(e1, e2) {
  if (e1.horiz === e2.horiz) return false;
  const H = e1.horiz ? e1 : e2, V = e1.horiz ? e2 : e1;
  const cLo = Math.min(H.ac, H.bc), cHi = Math.max(H.ac, H.bc);
  const rLo = Math.min(V.ar, V.br), rHi = Math.max(V.ar, V.br);
  return cLo < V.ac && V.ac < cHi && rLo < H.ar && H.ar < rHi;
}

// Count solutions consistent with the island clues, stopping at `limit`.
function countSolutions(islands, limit) {
  const edges = candidateEdges(islands);
  const inc = islands.map(() => []);
  edges.forEach((e, i) => { inc[e.a].push(i); inc[e.b].push(i); });
  const cross = edges.map(() => []);
  for (let i = 0; i < edges.length; i++) {
    for (let j = i + 1; j < edges.length; j++) {
      if (edgesCross(edges[i], edges[j])) { cross[i].push(j); cross[j].push(i); }
    }
  }
  const val = new Array(edges.length).fill(0);
  const sum = islands.map(() => 0);
  const cap = islands.map((_, i) => 2 * inc[i].length);
  let found = 0;

  function connected() {
    const par = islands.map((_, i) => i);
    const find = x => { while (par[x] !== x) x = par[x] = par[par[x]]; return x; };
    edges.forEach((e, i) => { if (val[i] > 0) par[find(e.a)] = find(e.b); });
    const root = find(0);
    return islands.every((_, i) => find(i) === root);
  }
  function bt(i) {
    if (found >= limit) return;
    if (i === edges.length) {
      if (islands.every((is, k) => sum[k] === is.n) && connected()) found++;
      return;
    }
    const e = edges[i];
    for (let v = 0; v <= 2; v++) {
      if (v > 0 && cross[i].some(j => j < i && val[j] > 0)) continue;
      val[i] = v;
      sum[e.a] += v; sum[e.b] += v;
      cap[e.a] -= 2; cap[e.b] -= 2;
      const okA = sum[e.a] <= islands[e.a].n && sum[e.a] + cap[e.a] >= islands[e.a].n;
      const okB = sum[e.b] <= islands[e.b].n && sum[e.b] + cap[e.b] >= islands[e.b].n;
      if (okA && okB) bt(i + 1);
      sum[e.a] -= v; sum[e.b] -= v;
      cap[e.a] += 2; cap[e.b] += 2;
      if (found >= limit) { val[i] = 0; return; }
    }
    val[i] = 0;
  }
  bt(0);
  return found;
}

// Grow one connected island network on the grid; returns islands + edges.
function tryGenerate(p, rng) {
  const { w, h } = p;
  const grid = new Int8Array(w * h);   // 0 empty, 1 island, 2 h-bridge, 3 v-bridge
  const islands = [];
  const edges = [];
  const edgeSet = new Set();
  function place(r, c) { grid[r * w + c] = 1; islands.push({ r, c }); return islands.length - 1; }

  place(2 + ((rng() * (h - 4)) | 0), 2 + ((rng() * (w - 4)) | 0));

  let tries = 0;
  while (islands.length < p.count && tries < 4000) {
    tries++;
    const A = islands[(rng() * islands.length) | 0];
    const d = DIRS[(rng() * 4) | 0];
    const maxL = 2 + ((rng() * 4) | 0);
    for (let L = 2; L <= maxL; L++) {
      const tr = A.r + d[0] * L, tc = A.c + d[1] * L;
      if (tr < 0 || tc < 0 || tr >= h || tc >= w) break;
      if (grid[tr * w + tc] !== 0) break;
      const cells = [];
      let clear = true;
      for (let k = 1; k < L; k++) {
        const ci = (A.r + d[0] * k) * w + (A.c + d[1] * k);
        if (grid[ci] !== 0) { clear = false; break; }
        cells.push(ci);
      }
      if (!clear) continue;
      const ai = islands.indexOf(A);
      const bi = place(tr, tc);
      const orient = d[0] === 0 ? 2 : 3;
      cells.forEach(ci => { grid[ci] = orient; });
      const bridges = rng() < 0.5 ? 1 : 2;
      edges.push({ a: ai, b: bi, bridges });
      edgeSet.add(ai + '-' + bi);
      break;
    }
  }
  if (islands.length < p.count) return null;

  // extra edges between aligned islands with a fully clear corridor (adds cycles)
  for (const cand of candidateEdges(islands)) {
    const key = Math.min(cand.a, cand.b) + '-' + Math.max(cand.a, cand.b);
    if (edgeSet.has(cand.a + '-' + cand.b) || edgeSet.has(cand.b + '-' + cand.a)) continue;
    const stepR = Math.sign(cand.br - cand.ar), stepC = Math.sign(cand.bc - cand.ac);
    let r = cand.ar + stepR, c = cand.ac + stepC, clear = true;
    const cells = [];
    while (r !== cand.br || c !== cand.bc) {
      if (grid[r * w + c] !== 0) { clear = false; break; }
      cells.push(r * w + c);
      r += stepR; c += stepC;
    }
    if (!clear || rng() < 0.45) continue;
    const orient = cand.horiz ? 2 : 3;
    cells.forEach(ci => { grid[ci] = orient; });
    edges.push({ a: cand.a, b: cand.b, bridges: rng() < 0.5 ? 1 : 2 });
    edgeSet.add(key);
  }

  const clue = islands.map(() => 0);
  for (const e of edges) { clue[e.a] += e.bridges; clue[e.b] += e.bridges; }
  islands.forEach((is, i) => { is.n = clue[i]; });
  if (islands.some(is => is.n === 0 || is.n > 8)) return null;
  return { w, h, islands, solution: edges };
}

// Build a uniquely-solvable bridges puzzle for a level from its seed.
function buildPuzzle(p) {
  const rng = seededRandom(p.seed);
  for (let attempt = 0; attempt < 600; attempt++) {
    const res = tryGenerate(p, rng);
    if (res && countSolutions(res.islands, 2) === 1) {
      res.edges = candidateEdges(res.islands);
      return res;
    }
  }
  return null;
}
