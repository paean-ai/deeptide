// Pixel Fit - a polyomino packing puzzle.
//
// A rectangular frame is cut into a set of irregular pieces. The pieces are
// shuffled into a tray, each spun to a random rotation. Rotate and place
// every piece back so the frame is filled exactly - no gaps, no overlaps.
//
// Each level is { w, h, k, seed }. The frame is partitioned into k connected
// pieces, so a solution is guaranteed to exist by construction.

const VW = 360, VH = 480;

const LEVELS = [
  { name: ['Hatch',   '小格'], w: 4, h: 4, k: 4, seed: 311 },
  { name: ['Crate',   '木箱'], w: 5, h: 4, k: 5, seed: 421 },
  { name: ['Lattice', '格栅'], w: 5, h: 5, k: 6, seed: 547 },
  { name: ['Parquet', '拼木'], w: 6, h: 5, k: 7, seed: 653 },
  { name: ['Mosaic',  '镶嵌'], w: 6, h: 6, k: 8, seed: 769 },
  { name: ['Foundry', '铸造'], w: 7, h: 6, k: 9, seed: 881 },
];
const LEVEL_COUNT = LEVELS.length;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () { s = (s * 16807) % 2147483647; return (s - 1) / 2147483646; };
}

// ---- frame partition ---------------------------------------------------
// Round-robin BFS expansion from k seed cells. Reject if any region is
// smaller than 3 cells or larger than 9 (degenerate / unwieldy pieces).
function partition(w, h, k, rng) {
  const total = w * h;
  const seeds = new Set();
  while (seeds.size < k) seeds.add((rng() * total) | 0);
  const owner = new Array(total).fill(-1);
  const fronts = [];
  let id = 0;
  for (const s of seeds) { owner[s] = id; fronts.push([s]); id++; }
  let remaining = total - k;
  while (remaining > 0) {
    let progressed = false;
    for (let r = 0; r < k; r++) {
      if (!fronts[r].length) continue;
      const cands = [];
      for (const c of fronts[r]) {
        const x = c % w, y = (c / w) | 0;
        for (const [dx, dy] of [[1,0],[-1,0],[0,1],[0,-1]]) {
          const nx = x + dx, ny = y + dy;
          if (nx < 0 || nx >= w || ny < 0 || ny >= h) continue;
          const ni = ny * w + nx;
          if (owner[ni] === -1) cands.push(ni);
        }
      }
      if (!cands.length) { fronts[r].length = 0; continue; }
      const pick = cands[(rng() * cands.length) | 0];
      owner[pick] = r;
      fronts[r].push(pick);
      remaining--;
      progressed = true;
      if (remaining <= 0) break;
    }
    if (!progressed) return null;
  }
  const regions = Array.from({ length: k }, () => []);
  for (let i = 0; i < total; i++) regions[owner[i]].push(i);
  for (const reg of regions) if (reg.length < 3 || reg.length > 9) return null;
  return regions;
}

// ---- piece geometry ----------------------------------------------------
// Sort cells row-major and shift so the first cell sits at (0,0)-ish; the
// first cell (topmost, then leftmost) is the placement anchor.
function normalize(cells) {
  let minX = Infinity, minY = Infinity;
  for (const [x, y] of cells) { if (x < minX) minX = x; if (y < minY) minY = y; }
  const out = cells.map(([x, y]) => [x - minX, y - minY]);
  out.sort((a, b) => a[1] - b[1] || a[0] - b[0]);
  return out;
}

// Rotate a piece 90 degrees clockwise.
function rotateCells(cells) {
  return normalize(cells.map(([x, y]) => [-y, x]));
}

function spin(cells, times) {
  let c = cells;
  for (let i = 0; i < (times & 3); i++) c = rotateCells(c);
  return c;
}

// ---- build -------------------------------------------------------------
const PIECE_COLORS = ['#e8554f', '#f0883a', '#f4c84a', '#5fc06e',
                      '#46b8c4', '#5f86e0', '#a06fd0', '#e87aa8'];

function buildGame(levelIndex) {
  const cfg = LEVELS[levelIndex];
  let regions = null;
  for (let attempt = 0; attempt < 400 && !regions; attempt++) {
    const rng = seededRandom(cfg.seed + attempt * 1009);
    regions = partition(cfg.w, cfg.h, cfg.k, rng);
  }
  // Build pieces from the regions; spin each to a deterministic rotation.
  const spinRng = seededRandom(cfg.seed * 7 + 13);
  const pieces = regions.map((reg, i) => {
    const cells = normalize(reg.map(c => [c % cfg.w, (c / cfg.w) | 0]));
    return {
      id: i,
      cells: spin(cells, 1 + ((spinRng() * 3) | 0)),   // never the solved rotation
      color: PIECE_COLORS[i % PIECE_COLORS.length],
      placed: null,                                     // {col, row} once on the frame
    };
  });
  return {
    levelIndex, cfg,
    w: cfg.w, h: cfg.h,
    pieces,
    occ: new Array(cfg.w * cfg.h).fill(-1),              // cell -> piece id
    selected: -1,
    moves: 0,
    over: false,
  };
}

// Absolute frame cells a piece would occupy if its anchor (cells[0]) lands
// on (col, row).
function placedCells(piece, col, row) {
  const a = piece.cells[0];
  return piece.cells.map(([x, y]) => [col + x - a[0], row + y - a[1]]);
}

function canPlace(s, pieceIdx, col, row) {
  const cells = placedCells(s.pieces[pieceIdx], col, row);
  for (const [x, y] of cells) {
    if (x < 0 || x >= s.w || y < 0 || y >= s.h) return false;
    if (s.occ[y * s.w + x] !== -1) return false;
  }
  return true;
}

function place(s, pieceIdx, col, row) {
  if (s.over || !canPlace(s, pieceIdx, col, row)) return false;
  const p = s.pieces[pieceIdx];
  for (const [x, y] of placedCells(p, col, row)) s.occ[y * s.w + x] = p.id;
  p.placed = { col, row };
  s.moves++;
  if (s.pieces.every(pc => pc.placed)) s.over = true;
  return true;
}

function pickUp(s, pieceIdx) {
  const p = s.pieces[pieceIdx];
  if (!p.placed) return false;
  for (let i = 0; i < s.occ.length; i++) if (s.occ[i] === p.id) s.occ[i] = -1;
  p.placed = null;
  s.over = false;
  return true;
}

// Rotate a tray (unplaced) piece in place.
function rotatePiece(s, pieceIdx) {
  const p = s.pieces[pieceIdx];
  if (p.placed) return false;
  p.cells = rotateCells(p.cells);
  return true;
}

function cellOwner(s, col, row) {
  if (col < 0 || col >= s.w || row < 0 || row >= s.h) return -1;
  return s.occ[row * s.w + col];
}

function isSolved(s) { return s.pieces.every(p => p.placed); }
