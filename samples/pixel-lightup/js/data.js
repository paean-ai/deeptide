// Pixel Light Up - Akari puzzles: generation, clue derivation, evaluation.
//
// Each puzzle is a hand-built wall layout. A solver enumerates every valid
// light arrangement (all cells lit, no bulb lighting another), then picks the
// one whose wall-adjacency numbers are UNIQUE among all arrangements - so the
// numbered puzzle that ships has exactly one solution.

const VW = 360, VH = 480;

const LAYOUTS = [
  { name: ['Foyer', '门厅'], rows: [
    '...#...',
    '.#...#.',
    '.......',
    '#..#..#',
    '.......',
    '.#...#.',
    '...#...',
  ] },
  { name: ['Gallery', '画廊'], rows: [
    '..#..#.',
    '.......',
    '#...#..',
    '..#....',
    '..#...#',
    '.......',
    '.#..#..',
  ] },
  { name: ['Parlor', '客厅'], rows: [
    '.#...#.',
    '...#...',
    '#.....#',
    '..#.#..',
    '#.....#',
    '...#...',
    '.#...#.',
  ] },
  { name: ['Atrium', '中庭'], rows: [
    '...#..#.',
    '.#......',
    '......#.',
    '..#..#..',
    '..#..#..',
    '.#......',
    '.#..#...',
    '...#..#.',
  ] },
  { name: ['Wing', '侧厅'], rows: [
    '.#...#..',
    '....#...',
    '.#....#.',
    '...#....',
    '....#...',
    '.#....#.',
    '...#....',
    '..#...#.',
  ] },
  { name: ['Manor', '庄园'], rows: [
    '..#...#.',
    '.....#..',
    '.#......',
    '...#..#.',
    '.#......',
    '..#..#..',
    '.#.....#',
    '...#..#.',
  ] },
  { name: ['Library', '书房'], rows: [
    '..#...#',
    '.......',
    '#.....#',
    '...#...',
    '#.....#',
    '.......',
    '#...#..',
  ] },
  { name: ['Conservatory', '花房'], rows: [
    '#......',
    '.#...#.',
    '..#....',
    '....#..',
    '..#....',
    '.#...#.',
    '......#',
  ] },
  { name: ['Study', '书斋'], rows: [
    '....#..',
    '..#....',
    '.......',
    '#..#..#',
    '.......',
    '....#..',
    '..#....',
  ] },
  { name: ['Cellar', '地窖'], rows: [
    '#..#..#',
    '.......',
    '..#.#..',
    '.......',
    '..#.#..',
    '.......',
    '#..#..#',
  ] },
  { name: ['Ballroom', '舞厅'], rows: [
    '..#...#.',
    '.#......',
    '......#.',
    '...##...',
    '...##...',
    '.#......',
    '......#.',
    '.#...#..',
  ] },
  { name: ['Pantry', '食品间'], rows: [
    '.#....#.',
    '....#...',
    '#......#',
    '..#..#..',
    '..#..#..',
    '#......#',
    '...#....',
    '.#....#.',
  ] },
  { name: ['Vault', '金库'], rows: [
    '#..#..#.',
    '........',
    '.#....#.',
    '..#..#..',
    '..#..#..',
    '.#....#.',
    '........',
    '.#..#..#',
  ] },
];
const PUZZLE_COUNT = LAYOUTS.length;
const ADJ = [[0, -1], [0, 1], [-1, 0], [1, 0]];

// Parse a wall layout into geometry: walls, white cells, and the light
// segments (maximal white runs) each white cell belongs to.
function parseLayout(rows) {
  const h = rows.length, w = rows[0].length;
  const wall = rows.map(r => r.split('').map(ch => ch === '#'));
  const hSeg = [], vSeg = [];
  for (let r = 0; r < h; r++) { hSeg.push(new Array(w).fill(-1)); vSeg.push(new Array(w).fill(-1)); }
  let hN = 0;
  for (let r = 0; r < h; r++) {
    let id = -1;
    for (let c = 0; c < w; c++) {
      if (wall[r][c]) { id = -1; continue; }
      if (id < 0) id = hN++;
      hSeg[r][c] = id;
    }
  }
  let vN = 0;
  for (let c = 0; c < w; c++) {
    let id = -1;
    for (let r = 0; r < h; r++) {
      if (wall[r][c]) { id = -1; continue; }
      if (id < 0) id = vN++;
      vSeg[r][c] = id;
    }
  }
  const whiteCells = [], wallCells = [];
  for (let r = 0; r < h; r++) for (let c = 0; c < w; c++) {
    (wall[r][c] ? wallCells : whiteCells).push([r, c]);
  }
  return { w, h, wall, hSeg, vSeg, hN, vN, whiteCells, wallCells };
}

// Every valid light arrangement (capped): all white lit, no two bulbs share a
// segment. Returns arrays of booleans indexed by whiteCells.
function enumerate(P, cap) {
  const wc = P.whiteCells;
  const hBulb = new Array(P.hN).fill(false);
  const vBulb = new Array(P.vN).fill(false);
  const bulb = new Array(wc.length).fill(false);
  const sols = [];
  function bt(i) {
    if (sols.length >= cap) return;
    if (i === wc.length) {
      for (let k = 0; k < wc.length; k++) {
        const [r, c] = wc[k];
        if (!hBulb[P.hSeg[r][c]] && !vBulb[P.vSeg[r][c]]) return;
      }
      sols.push(bulb.slice());
      return;
    }
    const [r, c] = wc[i];
    const hid = P.hSeg[r][c], vid = P.vSeg[r][c];
    bulb[i] = false;
    bt(i + 1);
    if (sols.length >= cap) return;
    if (!hBulb[hid] && !vBulb[vid]) {
      hBulb[hid] = vBulb[vid] = bulb[i] = true;
      bt(i + 1);
      hBulb[hid] = vBulb[vid] = bulb[i] = false;
    }
  }
  bt(0);
  return sols;
}

function bulbGrid(P, sol) {
  const g = {};
  P.whiteCells.forEach(([r, c], i) => { if (sol[i]) g[r + ',' + c] = 1; });
  return g;
}
function wallNumber(P, bg, r, c) {
  let n = 0;
  for (const [dr, dc] of ADJ) if (bg[(r + dr) + ',' + (c + dc)]) n++;
  return n;
}

// Count solutions of a numbered puzzle (lit + no-mutual + exact wall numbers),
// stopping at `limit`. The numbers prune the search hard.
function solveNumbered(P, number, limit) {
  const wc = P.whiteCells;
  const wIndex = {};
  wc.forEach(([r, c], i) => { wIndex[r + ',' + c] = i; });
  const walls = [];
  for (const [r, c] of P.wallCells) {
    if (number[r][c] < 0) continue;
    const adj = [];
    for (const [dr, dc] of ADJ) {
      const k = (r + dr) + ',' + (c + dc);
      if (k in wIndex) adj.push(wIndex[k]);
    }
    walls.push({ target: number[r][c], adj });
  }
  const cellWalls = wc.map(() => []);
  walls.forEach((w, wi) => w.adj.forEach(ci => cellWalls[ci].push(wi)));
  const cur = walls.map(() => 0);
  const rem = walls.map(w => w.adj.length);
  const hBulb = new Array(P.hN).fill(false);
  const vBulb = new Array(P.vN).fill(false);
  let found = 0;

  function bt(i) {
    if (found >= limit) return;
    if (i === wc.length) {
      for (let k = 0; k < wc.length; k++) {
        const [r, c] = wc[k];
        if (!hBulb[P.hSeg[r][c]] && !vBulb[P.vSeg[r][c]]) return;
      }
      found++;
      return;
    }
    const [r, c] = wc[i];
    const hid = P.hSeg[r][c], vid = P.vSeg[r][c];
    const mw = cellWalls[i];
    // option: no bulb
    let ok = true;
    for (const wi of mw) { rem[wi]--; if (cur[wi] + rem[wi] < walls[wi].target) ok = false; }
    if (ok) bt(i + 1);
    for (const wi of mw) rem[wi]++;
    if (found >= limit) return;
    // option: bulb
    if (!hBulb[hid] && !vBulb[vid]) {
      ok = true;
      for (const wi of mw) {
        cur[wi]++; rem[wi]--;
        if (cur[wi] > walls[wi].target || cur[wi] + rem[wi] < walls[wi].target) ok = false;
      }
      if (ok) {
        hBulb[hid] = vBulb[vid] = true;
        bt(i + 1);
        hBulb[hid] = vBulb[vid] = false;
      }
      for (const wi of mw) { cur[wi]--; rem[wi]++; }
    }
  }
  bt(0);
  return found;
}

// Build a uniquely-solvable Akari puzzle for a layout: take candidate light
// arrangements, number every wall from one, keep the first whose numbered
// puzzle the constrained solver proves unique.
function buildPuzzle(layout) {
  const P = parseLayout(layout.rows);
  const cands = enumerate(P, 500);
  for (const S of cands) {
    const bg = bulbGrid(P, S);
    const number = [];
    for (let r = 0; r < P.h; r++) number.push(new Array(P.w).fill(-1));
    for (const [r, c] of P.wallCells) number[r][c] = wallNumber(P, bg, r, c);
    if (solveNumbered(P, number, 2) === 1) {
      const solution = P.whiteCells.filter((_, k) => S[k]).map(([r, c]) => r + ',' + c);
      return { w: P.w, h: P.h, wall: P.wall, number, solution, whiteCells: P.whiteCells };
    }
  }
  return null;
}

// Evaluate a player's bulb set: lit cells, conflicts, wall satisfaction, win.
function evaluate(pz, bulbs) {
  const lit = {};
  const conflict = new Set();
  for (const key of bulbs) {
    const [r, c] = key.split(',').map(Number);
    lit[key] = true;
    for (const [dr, dc] of ADJ) {
      let rr = r + dr, cc = c + dc;
      while (rr >= 0 && cc >= 0 && rr < pz.h && cc < pz.w && !pz.wall[rr][cc]) {
        const k = rr + ',' + cc;
        lit[k] = true;
        if (bulbs.has(k)) { conflict.add(key); conflict.add(k); }
        rr += dr; cc += dc;
      }
    }
  }
  let allLit = true;
  for (const [r, c] of pz.whiteCells) if (!lit[r + ',' + c]) { allLit = false; break; }
  const wallState = {};
  let wallsOk = true;
  for (let r = 0; r < pz.h; r++) for (let c = 0; c < pz.w; c++) {
    if (pz.number[r][c] < 0) continue;
    let n = 0;
    for (const [dr, dc] of ADJ) if (bulbs.has((r + dr) + ',' + (c + dc))) n++;
    wallState[r + ',' + c] = n;
    if (n !== pz.number[r][c]) wallsOk = false;
  }
  const solved = allLit && conflict.size === 0 && wallsOk;
  return { lit, conflict, wallState, solved };
}
