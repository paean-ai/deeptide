// Pixel One-Line - the one-stroke drawing puzzle (Chinese 一笔画). Each
// level is a graph of nodes joined by edges; trace a single continuous
// path that covers EVERY edge exactly once, never lifting the pen.
//
// A graph admits such a path (an Eulerian path) iff it is connected and
// has either 0 or 2 vertices of odd degree. Every level below is
// verified by the test harness to satisfy that — and to be solvable.

const VW = 360, VH = 480;

// Each level: nodes (pixel positions on a 0..1 normalised canvas, scaled
// at render time) + edges (pairs of node indices). Hand-designed so the
// shapes read clearly and each has an Eulerian path.
const LEVELS = [
  // 1. Envelope — the classic "can you draw it without lifting" shape.
  {
    name: ['Envelope', '信封'],
    nodes: [
      { x: 0.20, y: 0.30 }, { x: 0.80, y: 0.30 },   // 0 TL, 1 TR
      { x: 0.20, y: 0.70 }, { x: 0.80, y: 0.70 },   // 2 BL, 3 BR
      { x: 0.50, y: 0.10 },                          // 4 peak
    ],
    edges: [[0,1],[0,2],[1,3],[2,3],[0,3],[1,2],[0,4],[1,4]],
  },
  // 2. Hourglass — two triangles meeting at a waist.
  {
    name: ['Hourglass', '沙漏'],
    nodes: [
      { x: 0.25, y: 0.18 }, { x: 0.75, y: 0.18 },
      { x: 0.50, y: 0.50 },
      { x: 0.25, y: 0.82 }, { x: 0.75, y: 0.82 },
    ],
    edges: [[0,1],[0,2],[1,2],[2,3],[2,4],[3,4]],
  },
  // 3. Star — a five-point star drawn in one stroke (the famous one).
  {
    name: ['Star', '五角星'],
    nodes: [
      { x: 0.50, y: 0.10 }, { x: 0.88, y: 0.38 },
      { x: 0.73, y: 0.83 }, { x: 0.27, y: 0.83 },
      { x: 0.12, y: 0.38 },
    ],
    edges: [[0,2],[2,4],[4,1],[1,3],[3,0]],
  },
  // 4. Lattice — a 3-square strip; weave through the shared edges.
  {
    name: ['Lattice', '格网'],
    nodes: [
      { x: 0.15, y: 0.32 }, { x: 0.50, y: 0.32 }, { x: 0.85, y: 0.32 },
      { x: 0.15, y: 0.68 }, { x: 0.50, y: 0.68 }, { x: 0.85, y: 0.68 },
    ],
    edges: [[0,1],[1,2],[3,4],[4,5],[0,3],[1,4],[2,5],[0,4],[1,5]],
  },
  // 5. Crown — a zig-zag battlement with a base.
  {
    name: ['Crown', '皇冠'],
    nodes: [
      { x: 0.12, y: 0.62 }, { x: 0.30, y: 0.24 },
      { x: 0.44, y: 0.62 }, { x: 0.56, y: 0.24 },
      { x: 0.70, y: 0.62 }, { x: 0.88, y: 0.24 },
      { x: 0.50, y: 0.86 },
    ],
    edges: [[0,1],[1,2],[2,3],[3,4],[4,5],[0,6],[5,6],[0,2],[2,4]],
  },
  // 6. Fan — a hub with five spokes plus two outer chords. The hub has
  // odd degree 5 and node 5 has degree 1, so a one-stroke trail must
  // start at one and finish at the other — the trickiest route here.
  {
    name: ['Fan', '扇骨'],
    nodes: [
      { x: 0.50, y: 0.52 },                          // 0 hub
      { x: 0.50, y: 0.12 }, { x: 0.86, y: 0.34 },
      { x: 0.74, y: 0.84 }, { x: 0.26, y: 0.84 },
      { x: 0.14, y: 0.34 },
    ],
    edges: [
      [0,1],[0,2],[0,3],[0,4],[0,5],          // 5 spokes from the hub
      [1,2],[3,4],                            // two outer chords
    ],
  },
];
const LEVEL_COUNT = LEVELS.length;

// ---- graph helpers -----------------------------------------------------
function degreeOf(level, node) {
  let d = 0;
  for (const [a, b] of level.edges) { if (a === node) d++; if (b === node) d++; }
  return d;
}
function oddDegreeNodes(level) {
  const out = [];
  for (let i = 0; i < level.nodes.length; i++) {
    if (degreeOf(level, i) % 2 === 1) out.push(i);
  }
  return out;
}
function isConnected(level) {
  // Connected over the vertices that actually carry an edge.
  const adj = level.nodes.map(() => []);
  for (const [a, b] of level.edges) { adj[a].push(b); adj[b].push(a); }
  let start = -1;
  for (let i = 0; i < adj.length; i++) if (adj[i].length) { start = i; break; }
  if (start < 0) return false;
  const seen = new Set([start]);
  const stack = [start];
  while (stack.length) {
    const v = stack.pop();
    for (const n of adj[v]) if (!seen.has(n)) { seen.add(n); stack.push(n); }
  }
  for (let i = 0; i < adj.length; i++) if (adj[i].length && !seen.has(i)) return false;
  return true;
}
// A graph has an Eulerian path iff it is connected and has 0 or 2
// odd-degree vertices.
function hasEulerPath(level) {
  if (!isConnected(level)) return false;
  const odd = oddDegreeNodes(level).length;
  return odd === 0 || odd === 2;
}

// Adjacent edges of a node — returns [{edge index, other node}].
function edgesAt(level, node) {
  const out = [];
  level.edges.forEach((e, i) => {
    if (e[0] === node) out.push({ ei: i, to: e[1] });
    else if (e[1] === node) out.push({ ei: i, to: e[0] });
  });
  return out;
}

// ---- runtime state -----------------------------------------------------
function buildGame(levelIndex) {
  const lv = LEVELS[levelIndex];
  return {
    levelIndex, lv,
    used: new Array(lv.edges.length).fill(false),
    path: [],            // sequence of node indices visited
    current: -1,         // current pen node (-1 = pen not down yet)
    over: false, won: false,
  };
}

// Tap a node. If the pen isn't down, place it. Otherwise, if the tapped
// node is joined to the current node by an UNused edge, traverse it.
// Returns true if the state changed.
function tapNode(s, node) {
  if (s.over) return false;
  if (s.current === -1) {
    s.current = node;
    s.path = [node];
    return true;
  }
  // Find an unused edge between current and the tapped node.
  for (const { ei, to } of edgesAt(s.lv, s.current)) {
    if (to === node && !s.used[ei]) {
      s.used[ei] = true;
      s.current = node;
      s.path.push(node);
      if (s.used.every(Boolean)) { s.over = true; s.won = true; }
      return true;
    }
  }
  return false;
}

function undo(s) {
  if (s.path.length <= 1) {
    // Lift the pen entirely.
    if (s.current !== -1) { s.current = -1; s.path = []; return true; }
    return false;
  }
  const last = s.path.pop();
  const prev = s.path[s.path.length - 1];
  // Un-mark the edge between prev and last.
  for (const { ei, to } of edgesAt(s.lv, prev)) {
    if (to === last && s.used[ei]) { s.used[ei] = false; break; }
  }
  s.current = prev;
  s.over = false; s.won = false;
  return true;
}

function restart(s) {
  s.used = new Array(s.lv.edges.length).fill(false);
  s.path = [];
  s.current = -1;
  s.over = false; s.won = false;
}

function progress(s) {
  return s.used.filter(Boolean).length;
}
