// Pixel Shadow Heist - turn-based stealth levels.
//
// Each player move is a turn; every guard then advances one tile along its
// looping patrol. A guard's vision is a straight ray (length VISION) in its
// facing direction, blocked by walls. Stand in a ray or on a guard = caught.
//
// Grid legend: '#' wall   '.' floor   '@' start   'X' exit
// Guard patrols are explicit looping tile lists; consecutive tiles (and the
// last->first wrap) must be orthogonally adjacent.

const VISION = 3;

// Ping-pong patrol helpers.
function paceV(c, r1, r2) {
  const a = [];
  for (let r = r1; r <= r2; r++) a.push([c, r]);
  for (let r = r2 - 1; r > r1; r--) a.push([c, r]);
  return a;
}
function paceH(r, c1, c2) {
  const a = [];
  for (let c = c1; c <= c2; c++) a.push([c, r]);
  for (let c = c2 - 1; c > c1; c--) a.push([c, r]);
  return a;
}

const LEVELS = [
  { // 1 - one vertical patrol
    grid: [
      '#########',
      '#@......#',
      '#.......#',
      '#.......#',
      '#.......#',
      '#......X#',
      '#########',
    ],
    guards: [{ patrol: paceV(4, 1, 5) }],
  },
  { // 2 - two columns to thread
    grid: [
      '##########',
      '#@.......#',
      '#........#',
      '#........#',
      '#........#',
      '#.......X#',
      '##########',
    ],
    guards: [{ patrol: paceV(3, 1, 5) }, { patrol: paceV(6, 2, 5) }],
  },
  { // 3 - a horizontal sweeper
    grid: [
      '##########',
      '#@.......#',
      '#........#',
      '#........#',
      '#........#',
      '#.......X#',
      '##########',
    ],
    guards: [{ patrol: paceH(3, 1, 8) }],
  },
  { // 4 - cross patrols
    grid: [
      '###########',
      '#@........#',
      '#.........#',
      '#.........#',
      '#.........#',
      '#.........#',
      '#........X#',
      '###########',
    ],
    guards: [{ patrol: paceH(2, 1, 9) }, { patrol: paceV(5, 1, 6) }, { patrol: paceH(5, 2, 9) }],
  },
  { // 5 - a dividing wall
    grid: [
      '###########',
      '#@........#',
      '#....#....#',
      '#....#....#',
      '#....#....#',
      '#....#....#',
      '#........X#',
      '###########',
    ],
    guards: [{ patrol: paceV(2, 1, 5) }, { patrol: paceV(8, 2, 6) }],
  },
  { // 6 - patrol gauntlet
    grid: [
      '############',
      '#@.........#',
      '#..........#',
      '#..........#',
      '#..........#',
      '#..........#',
      '#..........#',
      '#.........X#',
      '############',
    ],
    guards: [{ patrol: paceV(3, 1, 7) }, { patrol: paceV(6, 1, 7) }, { patrol: paceV(9, 1, 7) }],
  },
  { // 7 - chambers
    grid: [
      '############',
      '#@...#.....#',
      '#....#.....#',
      '#....#.....#',
      '#..........#',
      '#....#.....#',
      '#....#.....#',
      '#....#....X#',
      '############',
    ],
    guards: [{ patrol: paceH(4, 1, 10) }, { patrol: paceV(8, 1, 7) }, { patrol: paceV(2, 1, 7) }],
  },
  { // 8 - the vault run
    grid: [
      '#############',
      '#@..........#',
      '#...........#',
      '#...........#',
      '#...........#',
      '#...........#',
      '#...........#',
      '#...........#',
      '#..........X#',
      '#############',
    ],
    guards: [
      { patrol: paceH(2, 1, 11) }, { patrol: paceV(6, 1, 8) },
      { patrol: paceH(6, 1, 11) }, { patrol: paceV(10, 1, 8) },
    ],
  },
];

const LEVEL_COUNT = LEVELS.length;

function parseHeist(rows) {
  const w = rows[0].length, h = rows.length;
  const walls = [];
  let start = { x: 1, y: 1 }, exit = { x: 1, y: 1 };
  for (let y = 0; y < h; y++) {
    walls.push([]);
    for (let x = 0; x < w; x++) {
      const ch = rows[y][x];
      walls[y].push(ch === '#');
      if (ch === '@') start = { x, y };
      if (ch === 'X') exit = { x, y };
    }
  }
  return { w, h, walls, start, exit };
}
