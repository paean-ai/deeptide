// Pixel Unblock - sliding-block puzzle: grids, parsing, BFS solver.

const VW = 360, VH = 480;
const GRID_N = 6;

// 6x6 grids. '.' empty. 'A' = the target block (2-wide, slides out the right
// edge of its row). Other letters are 2-3 cell blocks, horizontal or vertical.
const RAW_LEVELS = [
  [ '......', '......', 'AA..B.', '....B.', '......', '......' ],
  [ '..D...', '..D...', 'AA.B.C', '...B.C', '...B..', '......' ],
  [ '...D..', '...D..', 'AA.B..', '...B..', '...B..', '...FF.' ],
  [ '...B..', '...B..', 'AA.B..', '......', '...FF.', '...GG.' ],
  [ '...B..', '...B..', 'AA.B..', '..FF..', '..GG..', '..HH..' ],
  [ 'CDD.EE', 'C..F.G', 'AAHF.G', '..HF..', 'II.JJ.', '....KK' ],
  [ '..CC.D', 'EE...D', 'AAF..D', '..F.GG', '..FHH.', 'III...' ],
  [ '...B.E', '...B.E', 'AA.B.E', '..FF..', '.CGG..', '.C.HH.' ],
];

// Parse a grid into blocks: { x, y, len, horiz, letter }.
function parseLevel(grid) {
  const cellsBy = {};
  for (let y = 0; y < GRID_N; y++) {
    for (let x = 0; x < GRID_N; x++) {
      const ch = grid[y][x];
      if (ch === '.') continue;
      (cellsBy[ch] || (cellsBy[ch] = [])).push([x, y]);
    }
  }
  const blocks = [];
  let targetIdx = -1;
  for (const ch of Object.keys(cellsBy).sort()) {
    const cells = cellsBy[ch];
    const xs = cells.map(c => c[0]), ys = cells.map(c => c[1]);
    const horiz = new Set(ys).size === 1;
    const b = {
      letter: ch, len: cells.length, horiz,
      x: Math.min(...xs), y: Math.min(...ys),
    };
    if (ch === 'A') targetIdx = blocks.length;
    blocks.push(b);
  }
  return { blocks, targetIdx, exitRow: blocks[targetIdx].y };
}

// occupancy grid for a set of block positions (positions = anchor x/y array).
function occupancyOf(blocks, pos) {
  const g = Array.from({ length: GRID_N }, () => new Array(GRID_N).fill(false));
  for (let i = 0; i < blocks.length; i++) {
    const b = blocks[i];
    const x = b.horiz ? pos[i] : b.x;
    const y = b.horiz ? b.y : pos[i];
    for (let k = 0; k < b.len; k++) {
      g[b.horiz ? y : y + k][b.horiz ? x + k : x] = true;
    }
  }
  return g;
}
function startPos(level) {
  return level.blocks.map(b => b.horiz ? b.x : b.y);
}
// The target can leave when every cell right of it on its row is clear.
function isWin(level, pos) {
  const t = level.targetIdx, b = level.blocks[t];
  const g = occupancyOf(level.blocks, pos);
  for (let x = pos[t] + b.len; x < GRID_N; x++) {
    if (g[b.y][x]) return false;
  }
  return true;
}

// BFS: shortest move count to a win, or -1 if unsolvable.
function solveLevel(level) {
  const start = startPos(level);
  if (isWin(level, start)) return 0;
  const seen = new Set([start.join(',')]);
  let frontier = [start];
  for (let depth = 1; depth <= 60 && frontier.length; depth++) {
    const next = [];
    for (const pos of frontier) {
      const g = occupancyOf(level.blocks, pos);
      for (let i = 0; i < level.blocks.length; i++) {
        const b = level.blocks[i];
        // clear this block from g, then try sliding it
        for (let k = 0; k < b.len; k++) {
          if (b.horiz) g[b.y][pos[i] + k] = false;
          else g[pos[i] + k][b.x] = false;
        }
        const lo = 0, hi = GRID_N - b.len;
        for (const dir of [-1, 1]) {
          let p = pos[i] + dir;
          while (p >= lo && p <= hi) {
            // the newly entered cell must be free
            const ex = b.horiz ? (dir < 0 ? p : p + b.len - 1) : b.x;
            const ey = b.horiz ? b.y : (dir < 0 ? p : p + b.len - 1);
            if (g[ey][ex]) break;
            const np = pos.slice();
            np[i] = p;
            const key = np.join(',');
            if (!seen.has(key)) {
              seen.add(key);
              if (isWin(level, np)) return depth;
              next.push(np);
            }
            p += dir;
          }
        }
        // restore this block into g
        for (let k = 0; k < b.len; k++) {
          if (b.horiz) g[b.y][pos[i] + k] = true;
          else g[pos[i] + k][b.x] = true;
        }
      }
    }
    frontier = next;
  }
  return -1;
}

const LEVELS = RAW_LEVELS.map(parseLevel);
const LEVEL_COUNT = LEVELS.length;
