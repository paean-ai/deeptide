// Pixel Edgematch - an edge-matching tile puzzle.
//
// The board is fully tiled with square tiles, each carrying a colour on
// every edge. The tiles are scrambled - swapped around and spun. Swap and
// rotate them back so every shared edge has the same colour on both sides
// (and the grey border colour faces out).
//
// Each level is { w, h, colors, seed }. The board is built from a solved
// colouring, so a solution is guaranteed by construction.

const VW = 360, VH = 480;

// Edge colour 0 is the neutral border colour; 1..K are puzzle colours.
const LEVELS = [
  { name: ['Hearth',  '炉边'], w: 3, h: 3, colors: 3, seed: 311 },
  { name: ['Parlor',  '客厅'], w: 4, h: 3, colors: 3, seed: 427 },
  { name: ['Gallery', '画廊'], w: 4, h: 4, colors: 4, seed: 541 },
  { name: ['Atrium',  '中庭'], w: 5, h: 4, colors: 4, seed: 659 },
  { name: ['Rotunda', '圆厅'], w: 5, h: 5, colors: 5, seed: 773 },
  { name: ['Cathedral', '大殿'], w: 6, h: 5, colors: 5, seed: 887 },
];
const LEVEL_COUNT = LEVELS.length;

// Edge directions on a tile.
const TOP = 0, RIGHT = 1, BOTTOM = 2, LEFT = 3;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () { s = (s * 16807) % 2147483647; return (s - 1) / 2147483646; };
}
function shuffle(a, rng) {
  for (let i = a.length - 1; i > 0; i--) {
    const j = (rng() * (i + 1)) | 0;
    const t = a[i]; a[i] = a[j]; a[j] = t;
  }
  return a;
}

// A tile's edge colour in board direction `dir`, given its rotation.
// Rotating a tile clockwise by `rot` moves the base LEFT edge toward TOP.
function edgeOf(tile, dir) {
  return tile.base[(dir - tile.rot + 4) % 4];
}

function buildGame(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const w = cfg.w, h = cfg.h, K = cfg.colors, N = w * h;
  const rng = seededRandom(cfg.seed);
  // Random colour for every internal grid edge; the border is colour 0.
  // vEdge[r][c] = colour of the edge between cell (r,c) and (r,c+1).
  // hEdge[r][c] = colour of the edge between cell (r,c) and (r+1,c).
  const vEdge = [], hEdge = [];
  for (let r = 0; r < h; r++) {
    vEdge.push([]);
    for (let c = 0; c < w - 1; c++) vEdge[r].push(1 + ((rng() * K) | 0));
  }
  for (let r = 0; r < h - 1; r++) {
    hEdge.push([]);
    for (let c = 0; c < w; c++) hEdge[r].push(1 + ((rng() * K) | 0));
  }
  // The solved tile for each cell - its four edge colours.
  const solved = [];
  for (let r = 0; r < h; r++) {
    for (let c = 0; c < w; c++) {
      const top    = r === 0     ? 0 : hEdge[r - 1][c];
      const bottom = r === h - 1 ? 0 : hEdge[r][c];
      const left   = c === 0     ? 0 : vEdge[r][c - 1];
      const right  = c === w - 1 ? 0 : vEdge[r][c];
      solved.push([top, right, bottom, left]);
    }
  }
  // Scramble: spin each tile, then permute the tiles across the board.
  let tiles, grid;
  for (let attempt = 0; attempt < 50; attempt++) {
    tiles = solved.map(base => ({ base: base.slice(), rot: (rng() * 4) | 0 }));
    grid = shuffle(Array.from({ length: N }, (_, i) => i), rng);
    const s = { w, h, tiles, grid };
    if (!isSolved(s)) break;        // never hand the player a finished board
  }
  return {
    levelIndex, cfg, w, h, colors: K,
    tiles, grid,
    selected: -1,
    moves: 0,
    over: false,
  };
}

// ---- play --------------------------------------------------------------
function tileAt(s, cell) { return s.tiles[s.grid[cell]]; }

// Swap the tiles occupying two board cells.
function swapCells(s, a, b) {
  const t = s.grid[a]; s.grid[a] = s.grid[b]; s.grid[b] = t;
  s.moves++;
}

// Rotate the tile in a cell 90 degrees clockwise.
function rotateCell(s, cell) {
  tileAt(s, cell).rot = (tileAt(s, cell).rot + 1) % 4;
  s.moves++;
}

// Internal shared edges and whether each currently matches.
function edgeReport(s) {
  const out = [];
  for (let r = 0; r < s.h; r++) {
    for (let c = 0; c < s.w; c++) {
      const cell = r * s.w + c;
      if (c < s.w - 1) {
        const a = edgeOf(tileAt(s, cell), RIGHT);
        const b = edgeOf(tileAt(s, cell + 1), LEFT);
        out.push({ r, c, side: 'right', ok: a === b });
      }
      if (r < s.h - 1) {
        const a = edgeOf(tileAt(s, cell), BOTTOM);
        const b = edgeOf(tileAt(s, cell + s.w), TOP);
        out.push({ r, c, side: 'bottom', ok: a === b });
      }
    }
  }
  return out;
}

function matchedCount(s) {
  let n = 0;
  for (const e of edgeReport(s)) if (e.ok) n++;
  return n;
}

function isSolved(s) {
  return edgeReport(s).every(e => e.ok);
}
