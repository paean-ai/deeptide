// Pixel Slitherlink - the classic single-loop puzzle.
//
// Draw segments along the lattice so they form ONE closed loop with no
// branches and no stray pieces. A numbered cell must be bordered by
// exactly that many loop segments; a blank cell may have any number.
//
// Each level is { C, seed }. buildPuzzle is deterministic in the seed and
// returns { C, clues, graph, levelIndex, cfg }, where clues[i] is the cell
// clue (0..3) or -1 for a blank cell.

const VW = 360, VH = 480;

const LEVELS = [
  { name: ['Moat',       '护城河'], C: 5, seed: 311 },
  { name: ['Rampart',    '城垣'],   C: 5, seed: 437 },
  { name: ['Hedge',      '树篱'],   C: 6, seed: 541 },
  { name: ['Coil',       '缠环'],   C: 6, seed: 659 },
  { name: ['Meander',    '蜿蜒'],   C: 7, seed: 773 },
  { name: ['Serpentine', '盘蛇'],   C: 7, seed: 881 },
];
const LEVEL_COUNT = LEVELS.length;

// Edge states the player cycles through.
const E_BLANK = 0, E_LINE = 1, E_CROSS = 2;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () { s = (s * 16807) % 2147483647; return (s - 1) / 2147483646; };
}

// ---- the loop, built as the boundary of a simply-connected blob --------
function genBlob(C, rng, frac) {
  const inB = new Array(C * C).fill(false);
  inB[(rng() * C * C) | 0] = true;
  let count = 1;
  const target = Math.max(3, Math.round(C * C * frac));
  while (count < target) {
    const front = [];
    for (let i = 0; i < C * C; i++) {
      if (inB[i]) continue;
      const x = i % C, y = (i / C) | 0;
      for (const [dx, dy] of [[1,0],[-1,0],[0,1],[0,-1]]) {
        const nx = x + dx, ny = y + dy;
        if (nx >= 0 && nx < C && ny >= 0 && ny < C && inB[ny * C + nx]) { front.push(i); break; }
      }
    }
    if (!front.length) break;
    inB[front[(rng() * front.length) | 0]] = true;
    count++;
  }
  // Fill any enclosed holes so the blob is simply connected (single-loop boundary).
  const reach = new Array(C * C).fill(false), st = [];
  for (let i = 0; i < C * C; i++) {
    const x = i % C, y = (i / C) | 0;
    if ((x === 0 || y === 0 || x === C - 1 || y === C - 1) && !inB[i] && !reach[i]) {
      reach[i] = true; st.push(i);
    }
  }
  while (st.length) {
    const i = st.pop(), x = i % C, y = (i / C) | 0;
    for (const [dx, dy] of [[1,0],[-1,0],[0,1],[0,-1]]) {
      const nx = x + dx, ny = y + dy;
      if (nx >= 0 && nx < C && ny >= 0 && ny < C) {
        const ni = ny * C + nx;
        if (!inB[ni] && !reach[ni]) { reach[ni] = true; st.push(ni); }
      }
    }
  }
  for (let i = 0; i < C * C; i++) if (!inB[i] && !reach[i]) inB[i] = true;
  return inB;
}

// Full clue for every cell = loop segments on its 4 sides.
function fullClues(C, inB) {
  const cl = new Array(C * C);
  for (let i = 0; i < C * C; i++) {
    const x = i % C, y = (i / C) | 0;
    let n = 0;
    for (const [dx, dy] of [[1,0],[-1,0],[0,1],[0,-1]]) {
      const nx = x + dx, ny = y + dy;
      const nb = (nx < 0 || nx >= C || ny < 0 || ny >= C) ? false : inB[ny * C + nx];
      if (nb !== inB[i]) n++;
    }
    cl[i] = n;
  }
  return cl;
}

// ---- lattice graph -----------------------------------------------------
function buildGraph(C) {
  const VWv = C + 1, vid = (r, c) => r * VWv + c;
  const edges = [];
  const cellEdges = Array.from({ length: C * C }, () => []);
  const vertEdges = Array.from({ length: VWv * VWv }, () => []);
  for (let r = 0; r <= C; r++) for (let c = 0; c < C; c++) {
    const e = edges.length, cells = [];
    if (r > 0) cells.push((r - 1) * C + c);
    if (r < C) cells.push(r * C + c);
    edges.push({ kind: 'h', r, c, v1: vid(r, c), v2: vid(r, c + 1), cells });
    for (const cc of cells) cellEdges[cc].push(e);
    vertEdges[vid(r, c)].push(e); vertEdges[vid(r, c + 1)].push(e);
  }
  for (let r = 0; r < C; r++) for (let c = 0; c <= C; c++) {
    const e = edges.length, cells = [];
    if (c > 0) cells.push(r * C + (c - 1));
    if (c < C) cells.push(r * C + c);
    edges.push({ kind: 'v', r, c, v1: vid(r, c), v2: vid(r + 1, c), cells });
    for (const cc of cells) cellEdges[cc].push(e);
    vertEdges[vid(r, c)].push(e); vertEdges[vid(r + 1, c)].push(e);
  }
  // Cell-major edge order - keeps the uniqueness solver's clue pruning tight.
  const order = [], seen = new Set();
  for (let i = 0; i < C * C; i++) for (const e of cellEdges[i]) if (!seen.has(e)) { seen.add(e); order.push(e); }
  return { C, edges, cellEdges, vertEdges, nVerts: VWv * VWv, order };
}

// Trace the ON edges: are they exactly one closed loop?
function tracesSingleLoop(G, isOn) {
  const on = [];
  for (let e = 0; e < G.edges.length; e++) if (isOn[e]) on.push(e);
  if (!on.length) return false;
  const used = new Set();
  let edge = on[0]; used.add(edge);
  const startV = G.edges[edge].v1;
  let atV = G.edges[edge].v2;
  for (;;) {
    let nxt = -1;
    for (const ee of G.vertEdges[atV]) if (isOn[ee] && ee !== edge) { nxt = ee; break; }
    if (nxt < 0) return false;
    if (used.has(nxt)) return used.size === on.length && atV === startV;
    used.add(nxt);
    atV = (G.edges[nxt].v1 === atV) ? G.edges[nxt].v2 : G.edges[nxt].v1;
    edge = nxt;
    if (used.size > on.length) return false;
  }
}

// Count single-loop solutions satisfying the clues (clue -1 = blank), to `cap`.
function countSolutions(G, clues, cap) {
  const C = G.C, E = G.edges.length, ord = G.order;
  const val = new Array(E).fill(false);
  const cellOn = new Array(C * C).fill(0);
  const cellUn = clues.map((_, i) => G.cellEdges[i].length);
  const vDeg = new Array(G.nVerts).fill(0);
  const vUn = G.vertEdges.map(a => a.length);
  let count = 0;
  function rec(k) {
    if (count >= cap) return;
    if (k === E) {
      for (let i = 0; i < C * C; i++) if (clues[i] >= 0 && cellOn[i] !== clues[i]) return;
      for (let v = 0; v < G.nVerts; v++) if (vDeg[v] !== 0 && vDeg[v] !== 2) return;
      if (tracesSingleLoop(G, val)) count++;
      return;
    }
    const e = ord[k], ed = G.edges[e];
    for (let on = 0; on <= 1; on++) {
      val[e] = !!on;
      for (const cc of ed.cells) { cellUn[cc]--; if (on) cellOn[cc]++; }
      if (on) { vDeg[ed.v1]++; vDeg[ed.v2]++; }
      vUn[ed.v1]--; vUn[ed.v2]--;
      let ok = true;
      for (const cc of ed.cells) {
        if (clues[cc] >= 0 && (cellOn[cc] > clues[cc] || cellOn[cc] + cellUn[cc] < clues[cc])) { ok = false; break; }
      }
      if (ok) for (const vv of [ed.v1, ed.v2]) {
        if (vDeg[vv] > 2 || (vDeg[vv] === 1 && vUn[vv] === 0)) { ok = false; break; }
      }
      if (ok) rec(k + 1);
      val[e] = false;
      for (const cc of ed.cells) { cellUn[cc]++; if (on) cellOn[cc]--; }
      if (on) { vDeg[ed.v1]--; vDeg[ed.v2]--; }
      vUn[ed.v1]++; vUn[ed.v2]++;
      if (count >= cap) return;
    }
  }
  rec(0);
  return count;
}

// Drop clues at random while the puzzle stays uniquely solvable, stopping at
// a floor so the board never gets so sparse that verification slows down.
function thinClues(G, full, rng, floor) {
  const work = full.slice();
  let kept = work.length;
  const idx = [];
  for (let i = 0; i < work.length; i++) idx.push(i);
  for (let i = idx.length - 1; i > 0; i--) {
    const j = (rng() * (i + 1)) | 0;
    const t = idx[i]; idx[i] = idx[j]; idx[j] = t;
  }
  for (const i of idx) {
    if (kept <= floor) break;
    const keep = work[i];
    work[i] = -1;
    if (countSolutions(G, work, 2) === 1) kept--;
    else work[i] = keep;
  }
  return work;
}

function buildPuzzle(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const C = cfg.C;
  const G = buildGraph(C);
  for (let attempt = 0; attempt < 200; attempt++) {
    const rng = seededRandom(cfg.seed + attempt * 1009);
    const inB = genBlob(C, rng, 0.34 + rng() * 0.26);
    const bc = inB.reduce((a, b) => a + (b ? 1 : 0), 0);
    if (bc < 3 || bc >= C * C) continue;
    const full = fullClues(C, inB);
    if (countSolutions(G, full, 2) !== 1) continue;
    const clues = thinClues(G, full, rng, Math.round(C * C * 0.5));
    return { C, clues, graph: G, levelIndex, cfg };
  }
  return null;
}

// ---- live state --------------------------------------------------------
// Cells whose drawn-line count already exceeds the clue.
function clueViolations(puzzle, edgeState) {
  const G = puzzle.graph, bad = new Set();
  for (let i = 0; i < puzzle.C * puzzle.C; i++) {
    if (puzzle.clues[i] < 0) continue;
    let n = 0;
    for (const e of G.cellEdges[i]) if (edgeState[e] === E_LINE) n++;
    if (n > puzzle.clues[i]) bad.add(i);
  }
  return bad;
}

// Per-cell clue count currently drawn (for the tick / cross marks).
function clueCount(puzzle, edgeState, cell) {
  let n = 0;
  for (const e of puzzle.graph.cellEdges[cell]) if (edgeState[e] === E_LINE) n++;
  return n;
}

function isSolved(puzzle, edgeState) {
  const G = puzzle.graph;
  const isOn = edgeState.map(s => s === E_LINE);
  for (let i = 0; i < puzzle.C * puzzle.C; i++) {
    if (puzzle.clues[i] < 0) continue;
    let n = 0;
    for (const e of G.cellEdges[i]) if (isOn[e]) n++;
    if (n !== puzzle.clues[i]) return false;
  }
  for (let v = 0; v < G.nVerts; v++) {
    let d = 0;
    for (const e of G.vertEdges[v]) if (isOn[e]) d++;
    if (d !== 0 && d !== 2) return false;
  }
  return tracesSingleLoop(G, isOn);
}
