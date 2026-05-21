// Pixel Tilt - slide-the-crystals puzzle. Pure logic: levels, slide, play.
//
// Tilt the whole cavern and every crystal slides that way until a wall, the
// edge, or another crystal stops it - all crystals move at once. Land each
// crystal on its matching-colour goal pad. Each level's par is the shortest
// solution, found by an offline breadth-first search and baked in here; the
// test re-derives it to confirm every level is solvable in par.

const VW = 360, VH = 480;

// Each level: n grid, wall cells, crystal start cells, goal cells (crystal i
// pairs with goal i by index / colour), and the BFS-shortest par.
const LEVELS = [
  { name: ['Quartz', '石英'],   n: 5,
    walls: [2, 13, 14, 16, 17], crystals: [24, 5], goals: [19, 9], par: 6 },
  { name: ['Garnet', '石榴石'], n: 5,
    walls: [2, 8, 11, 13, 15, 21, 23], crystals: [1, 6], goals: [7, 4], par: 10 },
  { name: ['Amethyst', '紫晶'], n: 6,
    walls: [0, 3, 6, 16, 19, 28, 29, 35], crystals: [4, 14, 26], goals: [17, 11, 5], par: 13 },
  { name: ['Topaz', '黄玉'],    n: 6,
    walls: [1, 2, 4, 12, 15, 16, 17, 19, 22, 30], crystals: [27, 31, 14], goals: [23, 5, 8], par: 17 },
  { name: ['Sapphire', '蓝宝'], n: 6,
    walls: [4, 6, 7, 9, 11, 15, 21, 26, 27, 28], crystals: [33, 32, 19, 13], goals: [30, 22, 0, 18], par: 20 },
  { name: ['Diamond', '钻石'],  n: 7,
    walls: [1, 5, 6, 9, 13, 19, 21, 24, 28, 29, 31, 46], crystals: [20, 4, 39], goals: [41, 12, 23], par: 24 },
];
const LEVEL_COUNT = LEVELS.length;

const TILTS = ['U', 'D', 'L', 'R'];
const TILT_VEC = { U: [0, -1], D: [0, 1], L: [-1, 0], R: [1, 0] };

// Slide every crystal in `dir`. Crystals furthest along the tilt resolve
// first, so trailing crystals stack behind them. Returns the new positions.
function slide(n, wallSet, positions, dir) {
  const v = TILT_VEC[dir], dx = v[0], dy = v[1];
  const proj = (c) => (c % n) * dx + ((c / n | 0)) * dy;
  const order = positions.map((_, i) => i).sort((a, b) => proj(positions[b]) - proj(positions[a]));
  const occ = new Set();
  const res = positions.slice();
  for (const i of order) {
    let x = positions[i] % n, y = positions[i] / n | 0;
    while (true) {
      const nx = x + dx, ny = y + dy;
      if (nx < 0 || ny < 0 || nx >= n || ny >= n) break;
      const nc = ny * n + nx;
      if (wallSet.has(nc) || occ.has(nc)) break;
      x = nx; y = ny;
    }
    const c = y * n + x;
    res[i] = c;
    occ.add(c);
  }
  return res;
}

function isWonPos(level, pos) {
  for (let i = 0; i < pos.length; i++) if (pos[i] !== level.goals[i]) return false;
  return true;
}

// ---- play state ----------------------------------------------------------
function newPlay(levelIndex) {
  const level = LEVELS[levelIndex];
  return {
    levelIndex, level, n: level.n,
    wallSet: new Set(level.walls),
    pos: level.crystals.slice(),
    moves: 0, history: [], over: false, won: false,
  };
}

// returns true if the tilt actually moved a crystal (a null tilt costs nothing)
function tilt(s, dir) {
  if (s.over) return false;
  const next = slide(s.n, s.wallSet, s.pos, dir);
  let changed = false;
  for (let i = 0; i < next.length; i++) if (next[i] !== s.pos[i]) { changed = true; break; }
  if (!changed) return false;
  s.history.push(s.pos.slice());
  s.pos = next;
  s.moves++;
  if (isWonPos(s.level, s.pos)) { s.over = true; s.won = true; }
  return true;
}

function undo(s) {
  if (!s.history.length) return false;
  s.pos = s.history.pop();
  s.moves--;
  s.over = false; s.won = false;
  return true;
}

function restart(s) {
  s.pos = s.level.crystals.slice();
  s.moves = 0; s.history = []; s.over = false; s.won = false;
}

function stars(moves, par) {
  if (moves <= par) return 3;
  if (moves <= par + Math.max(3, Math.round(par * 0.35))) return 2;
  return 1;
}
