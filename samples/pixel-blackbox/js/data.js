// Pixel Black Box - a deduction puzzle. Atoms are hidden in a square
// grid; you fire probes in from the edges and read the result (a hit, a
// reflection, or a labelled exit on some other edge). From the pattern
// of probes you mark where you think the atoms are, then reveal.
//
// Probe rules (classic Black Box):
//   - The ray travels one cell at a time in its current direction.
//   - If the NEXT cell has an atom -> HIT (probe is absorbed).
//   - Else inspect the two cells perpendicular to the next cell:
//       * both have atoms      -> REFLECT (probe exits the same edge cell)
//       * exactly one          -> DEFLECT 90 deg AWAY from that atom
//       * neither              -> step forward into the next cell.
//   - A probe whose entry cell already has an atom on either side
//     adjacent to the entry edge is also a REFLECT (no movement).
//   - Once the ray leaves the grid the exit edge cell is the answer.

const VW = 360, VH = 480;

// ---- levels ------------------------------------------------------------
// 9 boards: 6x6 / 7x7 / 8x8 / 9x9 with 3 .. 7 atoms.
const LEVELS = [
  { name: ['Atom',     '微粒'], n: 6, atoms: 3, seed: 6103 },
  { name: ['Cluster',  '团簇'], n: 6, atoms: 4, seed: 6204 },
  { name: ['Nebula',   '星云'], n: 7, atoms: 4, seed: 7305 },
  { name: ['Quasar',   '类星体'], n: 7, atoms: 5, seed: 7406 },
  { name: ['Galaxy',   '星系'], n: 8, atoms: 5, seed: 8507 },
  { name: ['Singularity','奇点'], n: 8, atoms: 6, seed: 8608 },
  { name: ['Cosmos',   '寰宇'], n: 9, atoms: 5, seed: 9709 },
  { name: ['Void',     '虚空'], n: 9, atoms: 6, seed: 9810 },
  { name: ['Event Horizon', '视界'], n: 9, atoms: 7, seed: 9911 },
];
const LEVEL_COUNT = LEVELS.length;

// ---- helpers -----------------------------------------------------------
function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () { s = (s * 16807) % 2147483647; return (s - 1) / 2147483646; };
}

// Place `count` atoms in random distinct interior cells. We keep them
// off the outer ring so every entry direction has some interesting
// behaviour available — pure outer-ring atoms reflect every adjacent
// probe and feel cheap.
function placeAtoms(n, count, rng) {
  const cells = [];
  for (let y = 1; y < n - 1; y++) for (let x = 1; x < n - 1; x++) cells.push([x, y]);
  for (let i = cells.length - 1; i > 0; i--) {
    const j = Math.floor(rng() * (i + 1));
    [cells[i], cells[j]] = [cells[j], cells[i]];
  }
  return cells.slice(0, count);
}

// ---- probe simulation -------------------------------------------------
// Edge addressing: a probe enters at one of 4*n edge cells. We label
// them 0 .. 4n-1 starting at the top edge going left-to-right, then
// right edge top-to-bottom, then bottom edge right-to-left, then left
// edge bottom-to-top. Each edge index has a paired (entry cell, initial
// direction).
function edgeEntry(n, idx) {
  const e = idx % (4 * n);
  if (e < n)       return { x: e,         y: 0,        dx: 0,  dy:  1 }; // top -> down
  if (e < 2 * n)   return { x: n - 1,     y: e - n,    dx: -1, dy:  0 }; // right -> left
  if (e < 3 * n)   return { x: 3 * n - 1 - e, y: n - 1, dx: 0, dy: -1 }; // bottom -> up
                   return { x: 0,         y: 4 * n - 1 - e, dx: 1, dy: 0 }; // left -> right
}
// Reverse lookup: given an exit cell + outgoing direction, find the edge
// index of that exit slot.
function edgeIndexOf(n, x, y, dx, dy) {
  // The ray steps OUT of (x,y) in direction (dx,dy); its exit slot is
  // the edge index whose entry is that exit cell with REVERSED direction.
  for (let i = 0; i < 4 * n; i++) {
    const e = edgeEntry(n, i);
    if (e.x === x && e.y === y && e.dx === -dx && e.dy === -dy) return i;
  }
  return -1;
}

// Atom-presence at (x,y).
function hasAtom(atoms, x, y) {
  for (const a of atoms) if (a[0] === x && a[1] === y) return true;
  return false;
}

// Fire a probe; returns { kind: 'hit' | 'reflect' | 'pass', exit?: idx }.
function probe(n, atoms, edgeIdx) {
  const start = edgeEntry(n, edgeIdx);
  let x = start.x, y = start.y;
  let dx = start.dx, dy = start.dy;
  // Edge-reflect: if the first cell (entry cell) has an atom DIAGONALLY
  // adjacent to the entry side (i.e. an atom at the cell beside the entry
  // perpendicular to the heading), the probe reflects immediately.
  const pX = -dy, pY = dx;       // perpendicular unit
  if (hasAtom(atoms, x + pX, y + pY) || hasAtom(atoms, x - pX, y - pY)) {
    return { kind: 'reflect' };
  }
  // Step through the grid.
  // Safety cap: a probe cannot loop forever in a finite grid (proven by
  // Black Box rules), but cap anyway.
  for (let step = 0; step < 200; step++) {
    const nx = x + dx, ny = y + dy;
    // Off-grid -> exit.
    if (nx < 0 || ny < 0 || nx >= n || ny >= n) {
      const exitIdx = edgeIndexOf(n, x, y, dx, dy);
      if (exitIdx === edgeIdx) return { kind: 'reflect' };
      return { kind: 'pass', exit: exitIdx };
    }
    // Atom in the next cell?
    if (hasAtom(atoms, nx, ny)) return { kind: 'hit' };
    // Perpendicular neighbours of the NEXT cell.
    const pxN = nx + pX, pyN = ny + pY;
    const pxS = nx - pX, pyS = ny - pY;
    const left  = hasAtom(atoms, pxN, pyN);
    const right = hasAtom(atoms, pxS, pyS);
    if (left && right) {
      // Bounce back along the entry direction.
      dx = -dx; dy = -dy;
      // Don't advance; the next iteration steps backwards.
    } else if (left) {
      // Deflect away from the left-perp atom: rotate dir 90 deg toward right-perp.
      const ndx = -pX, ndy = -pY;   // rotate by -90 of perp = away from left
      dx = ndx; dy = ndy;
    } else if (right) {
      const ndx = pX, ndy = pY;
      dx = ndx; dy = ndy;
    } else {
      // Free pass; step.
      x = nx; y = ny;
    }
  }
  return { kind: 'reflect' }; // fallback
}

// ---- runtime state -----------------------------------------------------
// marks: 0 = blank, 1 = marked atom guess
function buildGame(levelIndex) {
  const lv = LEVELS[levelIndex];
  const rng = seededRandom(lv.seed);
  const atoms = placeAtoms(lv.n, lv.atoms, rng);
  const marks = Array.from({ length: lv.n }, () => Array(lv.n).fill(0));
  return {
    levelIndex, lv, n: lv.n, atoms,
    marks,
    fired: {},                 // edgeIdx -> probe result
    nextLabel: 1,              // next pass-through label
    edgeLabel: {},             // edgeIdx -> letter/number label
    over: false, revealed: false, score: 0,
  };
}

function fireProbe(s, edgeIdx) {
  if (s.over || s.fired[edgeIdx]) return null;
  const result = probe(s.n, s.atoms, edgeIdx);
  s.fired[edgeIdx] = result;
  if (result.kind === 'pass') {
    // Pair the entry + exit with a shared label letter.
    const label = labelFor(s.nextLabel++);
    s.edgeLabel[edgeIdx] = label;
    s.edgeLabel[result.exit] = label;
    s.fired[result.exit] = { kind: 'passOut', mate: edgeIdx, label };
  }
  return result;
}

function labelFor(n) {
  // A, B, ..., Z, then AA, AB, ...
  const A = 'A'.charCodeAt(0);
  if (n <= 26) return String.fromCharCode(A + n - 1);
  return String.fromCharCode(A + ((n - 1) / 26 | 0) - 1) + String.fromCharCode(A + (n - 1) % 26);
}

function toggleMark(s, x, y) {
  if (s.over || s.revealed) return;
  s.marks[y][x] ^= 1;
}

function markCount(s) {
  let c = 0;
  for (let y = 0; y < s.n; y++) for (let x = 0; x < s.n; x++) if (s.marks[y][x]) c++;
  return c;
}

function probeCount(s) { return Object.keys(s.fired).filter(k => s.fired[k].kind !== 'passOut').length; }

function reveal(s) {
  if (s.over) return s.score;
  s.revealed = true; s.over = true;
  // Score = 200 * correct marks - 100 * incorrect marks - 5 * probes.
  let correct = 0, wrong = 0;
  for (let y = 0; y < s.n; y++) for (let x = 0; x < s.n; x++) {
    if (s.marks[y][x]) {
      if (hasAtom(s.atoms, x, y)) correct++; else wrong++;
    }
  }
  s.score = Math.max(0, correct * 200 - wrong * 100 - probeCount(s) * 5);
  s.correctMarks = correct;
  s.wrongMarks = wrong;
  s.missedAtoms = s.lv.atoms - correct;
  s.solved = correct === s.lv.atoms && wrong === 0;
  return s.score;
}
