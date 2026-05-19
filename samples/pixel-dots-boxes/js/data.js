// Pixel Dots & Boxes - board rules and the heuristic AI.

const VW = 360, VH = 480;
const B = 5;                 // boxes per side (6x6 dots, 25 boxes)
const PLAYER = 1, AI = 2;

const DIFFICULTIES = [
  { name: ['Easy', '简单'],   level: 0 },
  { name: ['Medium', '普通'], level: 1 },
  { name: ['Hard', '困难'],   level: 2 },
];

// h[r][c]: horizontal edge, r in 0..B, c in 0..B-1
// v[r][c]: vertical edge,   r in 0..B-1, c in 0..B
function newState() {
  return {
    h: Array.from({ length: B + 1 }, () => new Array(B).fill(false)),
    v: Array.from({ length: B }, () => new Array(B + 1).fill(false)),
    boxes: Array.from({ length: B }, () => new Array(B).fill(0)),
  };
}
function cloneState(s) {
  return {
    h: s.h.map(r => r.slice()),
    v: s.v.map(r => r.slice()),
    boxes: s.boxes.map(r => r.slice()),
  };
}

// the 4 edges of box (br,bc), as drawn-or-not
function boxEdges(s, br, bc) {
  return [s.h[br][bc], s.h[br + 1][bc], s.v[br][bc], s.v[br][bc + 1]];
}
function boxCount(s, br, bc) {
  return boxEdges(s, br, bc).filter(Boolean).length;
}
function edgeDrawn(s, e) {
  return e.t === 0 ? s.h[e.r][e.c] : s.v[e.r][e.c];
}
function setEdge(s, e, val) {
  if (e.t === 0) s.h[e.r][e.c] = val; else s.v[e.r][e.c] = val;
}
// boxes touching an edge
function edgeBoxes(e) {
  const out = [];
  if (e.t === 0) {                         // horizontal
    if (e.r > 0) out.push([e.r - 1, e.c]);
    if (e.r < B) out.push([e.r, e.c]);
  } else {                                 // vertical
    if (e.c > 0) out.push([e.r, e.c - 1]);
    if (e.c < B) out.push([e.r, e.c]);
  }
  return out;
}
function legalEdges(s) {
  const out = [];
  for (let r = 0; r <= B; r++) for (let c = 0; c < B; c++) if (!s.h[r][c]) out.push({ t: 0, r, c });
  for (let r = 0; r < B; r++) for (let c = 0; c <= B; c++) if (!s.v[r][c]) out.push({ t: 1, r, c });
  return out;
}
// draw an edge for `player`; claim any boxes it completes. returns # completed.
function drawEdge(s, e, player) {
  setEdge(s, e, true);
  let done = 0;
  for (const [br, bc] of edgeBoxes(e)) {
    if (s.boxes[br][bc] === 0 && boxCount(s, br, bc) === 4) {
      s.boxes[br][bc] = player;
      done++;
    }
  }
  return done;
}
function isOver(s) {
  for (let r = 0; r < B; r++) for (let c = 0; c < B; c++) if (s.boxes[r][c] === 0) return false;
  return true;
}
function scores(s) {
  let p = 0, a = 0;
  for (let r = 0; r < B; r++) for (let c = 0; c < B; c++) {
    if (s.boxes[r][c] === PLAYER) p++; else if (s.boxes[r][c] === AI) a++;
  }
  return { player: p, ai: a };
}

// does drawing e complete at least one box?
function isCompleting(s, e) {
  return edgeBoxes(e).some(([br, bc]) => boxCount(s, br, bc) === 3);
}
// does drawing e leave a box on 3 edges (a gift to the opponent)?
function isSafe(s, e) {
  return !edgeBoxes(e).some(([br, bc]) => boxCount(s, br, bc) === 2);
}
// greedily count boxes the opponent can run after edge e is given away
function chainGift(s, e) {
  const sim = cloneState(s);
  drawEdge(sim, e, AI);
  let grabbed = 0, guard = 0;
  while (guard++ < 200) {
    const comp = legalEdges(sim).find(x => isCompleting(sim, x));
    if (!comp) break;
    grabbed += drawEdge(sim, comp, PLAYER);
  }
  return grabbed;
}

// pick the AI's edge for the given difficulty level
function aiPickEdge(s, level) {
  const edges = legalEdges(s);
  if (!edges.length) return null;
  const completing = edges.filter(e => isCompleting(s, e));
  if (completing.length) return completing[(Math.random() * completing.length) | 0];
  if (level >= 1) {
    const safe = edges.filter(e => isSafe(s, e));
    if (safe.length) return safe[(Math.random() * safe.length) | 0];
  }
  if (level >= 2) {
    let best = null, bestGift = Infinity;
    for (const e of edges) {
      const g = chainGift(s, e);
      if (g < bestGift) { bestGift = g; best = e; }
    }
    if (best) return best;
  }
  return edges[(Math.random() * edges.length) | 0];
}
