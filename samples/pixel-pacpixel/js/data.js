// Pixel Pac-Pixel - a tribute to Pac-Man. A small hand-designed maze
// with pellets, four corner power-pellets, and two ghosts. Eat every
// pellet to clear a level; ghosts kill on contact except briefly after
// you eat a power-pellet (then they're edible and return to home base).
//
// Cells: ' '=wall, '.'=pellet, 'o'=power-pellet, '-'=empty corridor,
// 'h'=ghost home, 'T'=tunnel cap (passable; player wraps to opposite cap).
//
// Coordinate system: x = col (0..COLS-1), y = row (0..ROWS-1).
// Cell = 20 px square. Board (COLS*CELL) by (ROWS*CELL) sits below HUD.

const VW = 360, VH = 480;
const CELL = 20;
const BOARD_OX = 0;
const BOARD_OY = 32;

// Maze: 17 cols x 20 rows = 340 x 400 px. HUD owns the top 32 px,
// the bottom 48 are status / back button.
const MAZE_ROWS = [
  '                 ',
  ' ............... ',
  ' .   . . . .   . ',
  ' .o. . . . . .o. ',
  ' . . . . . . . . ',
  ' ............... ',
  ' . .   . . .   . ',
  ' ... . . . . ... ',
  ' .   . . . . .  T',
  ' ... .  hhh  ... ',
  ' .   .  hhh  .  T',
  ' ... . . . . ... ',
  ' . .   . . .   . ',
  ' ............... ',
  ' . . . . . . . . ',
  ' .o. . . . . .o. ',
  ' .   . . . .   . ',
  ' ............... ',
  '                 ',
  '                 ',
];
const COLS = MAZE_ROWS[0].length;
const ROWS = MAZE_ROWS.length;

// ---- levels ------------------------------------------------------------
// Difficulty scales ghost speed and the post-power-pellet panic time.
const LEVELS = [
  { name: ['Wakka',  '咔咔'], pacSpeed: 4.4, ghostSpeed: 3.6, panic: 8.5 },
  { name: ['Chomp',  '大口'], pacSpeed: 4.6, ghostSpeed: 3.9, panic: 7.5 },
  { name: ['Frenzy', '狂奔'], pacSpeed: 4.8, ghostSpeed: 4.2, panic: 6.5 },
  { name: ['Maze',   '迷宫'], pacSpeed: 5.0, ghostSpeed: 4.5, panic: 5.5 },
  { name: ['Echo',   '回声'], pacSpeed: 5.2, ghostSpeed: 4.9, panic: 4.5 },
  { name: ['Finale', '终曲'], pacSpeed: 5.4, ghostSpeed: 5.4, panic: 3.5 },
];
const LEVEL_COUNT = LEVELS.length;

// ---- maze model --------------------------------------------------------
function isWall(maze, c, r) {
  if (r < 0 || r >= ROWS) return true;
  if (c < 0 || c >= COLS) return true;
  return maze[r][c] === ' ';
}
function isTunnel(maze, c, r) {
  return r >= 0 && r < ROWS && c >= 0 && c < COLS && maze[r][c] === 'T';
}
function ghostHomeCells() {
  const cells = [];
  for (let r = 0; r < ROWS; r++) for (let c = 0; c < COLS; c++)
    if (MAZE_ROWS[r][c] === 'h') cells.push([c, r]);
  return cells;
}
function findStart() {
  // Pac starts at the centre of the bottom-half corridor (row 17 col centre).
  return { col: 8, row: 17 };
}

// ---- runtime state -----------------------------------------------------
function buildGame(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const maze = MAZE_ROWS.map(r => r.split(''));
  // Convert all '.' / 'o' into pellets count + keep the symbol for rendering.
  let pellets = 0, power = 0;
  for (let r = 0; r < ROWS; r++) for (let c = 0; c < COLS; c++) {
    if (maze[r][c] === '.') pellets++;
    if (maze[r][c] === 'o') power++;
  }
  const start = findStart();
  const ghostHome = ghostHomeCells();
  // Two ghosts: red Blinky (chase Pac) + pink Pinky (chase a cell ahead).
  const ghosts = [
    spawnGhost('blinky', '#ff5a5a', ghostHome[0][0],     ghostHome[0][1]),
    spawnGhost('pinky',  '#ffaad8', ghostHome[ghostHome.length - 1][0], ghostHome[ghostHome.length - 1][1]),
  ];
  return {
    levelIndex, cfg, maze,
    pellets, pelletsLeft: pellets, power, powerLeft: power,
    pac: {
      col: start.col, row: start.row, x: start.col + 0.5, y: start.row + 0.5,
      dir: { x: 0, y: 0 }, next: { x: 0, y: 0 }, mouthT: 0, alive: true, hitFlash: 0,
    },
    ghosts,
    panicT: 0,                  // seconds of remaining panic (post-power)
    score: 0, lives: 2,
    elapsed: 0,
    over: false, won: false,
    flash: 0,
  };
}

function spawnGhost(name, color, c, r) {
  return {
    name, color, col: c, row: r, x: c + 0.5, y: r + 0.5,
    dir: { x: 0, y: -1 }, mode: 'home', homeT: 1.0 + Math.random() * 1.5,
  };
}

// ---- input -------------------------------------------------------------
// We queue the player's NEXT direction; Pac changes direction at the next
// cell-centre that allows it (so swipes feel responsive without juddering
// against walls).
function setDir(s, dx, dy) {
  if (!s.pac.alive) return;
  s.pac.next = { x: dx, y: dy };
  if (s.pac.dir.x === 0 && s.pac.dir.y === 0) {
    // First input also sets the immediate direction so Pac moves at all.
    if (canMove(s, s.pac.col, s.pac.row, dx, dy)) s.pac.dir = { x: dx, y: dy };
  }
}
function canMove(s, c, r, dx, dy) {
  return !isWall(s.maze, c + dx, r + dy);
}

// ---- tick --------------------------------------------------------------
function tick(s, dt) {
  if (s.over) return;
  s.elapsed += dt;
  s.flash = Math.max(0, s.flash - dt);
  if (s.pac.hitFlash > 0) s.pac.hitFlash = Math.max(0, s.pac.hitFlash - dt);
  s.pac.mouthT += dt;
  if (s.panicT > 0) {
    s.panicT -= dt;
    if (s.panicT <= 0) {
      for (const g of s.ghosts) if (g.mode === 'panic') g.mode = 'chase';
    }
  }
  movePac(s, dt);
  for (const g of s.ghosts) moveGhost(s, g, dt);
  checkPellet(s);
  checkGhostCollide(s);
}

function movePac(s, dt) {
  const p = s.pac;
  if (!p.alive) return;
  // Try the queued direction at the next half-cell.
  const cx = Math.floor(p.x), cy = Math.floor(p.y);
  const atCenterX = Math.abs(p.x - (cx + 0.5)) < 0.06;
  const atCenterY = Math.abs(p.y - (cy + 0.5)) < 0.06;
  if (atCenterX && atCenterY) {
    if (p.next.x || p.next.y) {
      if (canMove(s, cx, cy, p.next.x, p.next.y)) {
        p.dir = { x: p.next.x, y: p.next.y };
      }
    }
    // Block forward movement at a wall.
    if (p.dir.x || p.dir.y) {
      if (!canMove(s, cx, cy, p.dir.x, p.dir.y)) {
        p.dir = { x: 0, y: 0 };
      }
    }
  }
  // Advance.
  const sp = s.cfg.pacSpeed;
  p.x += p.dir.x * sp * dt;
  p.y += p.dir.y * sp * dt;
  // Tunnel wrap on the horizontal exits.
  if (p.x < 0)     p.x = COLS - 0.5;
  if (p.x > COLS)  p.x = 0.5;
  p.col = Math.floor(p.x); p.row = Math.floor(p.y);
}

function moveGhost(s, g, dt) {
  // Home / release timer
  if (g.mode === 'home') {
    g.homeT -= dt;
    if (g.homeT <= 0) {
      // Step up out of the home cells.
      g.row -= 1; g.y -= 1;
      g.mode = 'chase';
    }
    return;
  }
  const sp = s.cfg.ghostSpeed * (g.mode === 'panic' ? 0.7 : g.mode === 'eyes' ? 1.6 : 1.0);
  // Choose direction at each cell centre.
  const cx = Math.floor(g.x), cy = Math.floor(g.y);
  const atCenter = Math.abs(g.x - (cx + 0.5)) < 0.06 && Math.abs(g.y - (cy + 0.5)) < 0.06;
  if (atCenter) {
    g.col = cx; g.row = cy;
    g.dir = pickGhostDir(s, g);
  }
  g.x += g.dir.x * sp * dt;
  g.y += g.dir.y * sp * dt;
  // Tunnel wrap for ghosts too.
  if (g.x < 0)     g.x = COLS - 0.5;
  if (g.x > COLS)  g.x = 0.5;
  // Eyes return to home: when in a home cell, respawn as a chaser.
  if (g.mode === 'eyes' && s.maze[Math.floor(g.y)] && s.maze[Math.floor(g.y)][Math.floor(g.x)] === 'h') {
    g.mode = 'chase';
  }
}

function pickGhostDir(s, g) {
  // Pick direction at the cell centre. Don't reverse unless dead-end.
  // Targets: blinky -> pac; pinky -> 4 cells ahead of pac; panic -> random;
  // eyes -> nearest home cell.
  const target = ghostTarget(s, g);
  const options = [{x:0,y:-1},{x:1,y:0},{x:0,y:1},{x:-1,y:0}];
  const reverse = { x: -g.dir.x, y: -g.dir.y };
  const legal = options.filter(o =>
    !isWall(s.maze, g.col + o.x, g.row + o.y) &&
    !(o.x === reverse.x && o.y === reverse.y));
  // If no non-reverse option (dead end), allow reverse.
  const choices = legal.length ? legal : options.filter(o =>
    !isWall(s.maze, g.col + o.x, g.row + o.y));
  if (!choices.length) return g.dir;
  if (g.mode === 'panic') {
    return choices[Math.floor(Math.random() * choices.length)];
  }
  // Pick option minimising distance to target.
  let best = choices[0], bd = 1e9;
  for (const o of choices) {
    const nx = g.col + o.x, ny = g.row + o.y;
    const d = (nx - target.col) ** 2 + (ny - target.row) ** 2;
    if (d < bd) { bd = d; best = o; }
  }
  return best;
}

function ghostTarget(s, g) {
  if (g.mode === 'eyes') {
    const home = ghostHomeCells();
    return { col: home[0][0], row: home[0][1] };
  }
  if (g.name === 'blinky') return { col: s.pac.col, row: s.pac.row };
  // Pinky aims 4 cells ahead of Pac.
  const ahead = 4;
  return {
    col: s.pac.col + s.pac.dir.x * ahead,
    row: s.pac.row + s.pac.dir.y * ahead,
  };
}

function checkPellet(s) {
  const p = s.pac;
  if (!p.alive) return;
  const cx = Math.floor(p.x), cy = Math.floor(p.y);
  // Only count a pellet when Pac is near the cell centre.
  if (Math.abs(p.x - (cx + 0.5)) > 0.35 || Math.abs(p.y - (cy + 0.5)) > 0.35) return;
  const cell = s.maze[cy] && s.maze[cy][cx];
  if (cell === '.') {
    s.maze[cy][cx] = '-';
    s.pelletsLeft--;
    s.score += 10;
  } else if (cell === 'o') {
    s.maze[cy][cx] = '-';
    s.powerLeft--;
    s.score += 50;
    s.panicT = s.cfg.panic;
    for (const g of s.ghosts) if (g.mode === 'chase') g.mode = 'panic';
    s.flash = 0.25;
  }
  if (s.pelletsLeft === 0 && s.powerLeft === 0) {
    s.over = true; s.won = true; s.score += 200;
    s.flash = 0.5;
  }
}

function checkGhostCollide(s) {
  const p = s.pac;
  if (!p.alive) return;
  for (const g of s.ghosts) {
    if (g.mode === 'eyes' || g.mode === 'home') continue;
    if (Math.abs(g.x - p.x) < 0.45 && Math.abs(g.y - p.y) < 0.45) {
      if (g.mode === 'panic') {
        // Eat the ghost.
        g.mode = 'eyes';
        s.score += 200;
        s.flash = 0.3;
      } else {
        die(s);
        return;
      }
    }
  }
}

function die(s) {
  const p = s.pac;
  if (!p.alive) return;
  p.alive = false;
  p.hitFlash = 0.6;
  s.flash = 0.4;
  s.lives--;
  if (s.lives < 0) { s.over = true; s.won = false; return; }
  // Brief respawn beat handled by the game loop.
  p.respawn = 0.8;
}

function resetPac(s) {
  const start = findStart();
  s.pac.col = start.col; s.pac.row = start.row;
  s.pac.x = start.col + 0.5; s.pac.y = start.row + 0.5;
  s.pac.dir = { x: 0, y: 0 }; s.pac.next = { x: 0, y: 0 };
  s.pac.alive = true;
}

function finalScore(s) {
  return s.score + Math.max(0, s.lives) * 100;
}
