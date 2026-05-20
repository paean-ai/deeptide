// Pixel Centipede - top-down shooter on a grid. A snaking centipede
// zigzags down through a mushroom field from the top of the screen;
// the player at the bottom shoots straight up. Hit any segment and it
// turns into a mushroom — the centipede splits at that point into two
// independent worms. Spiders sometimes bounce through the player area
// for bonus points. Clear every segment to advance.

const VW = 360, VH = 480;

// Grid: 18 cols wide, 22 rows tall, cell = 20 px. 18*20 = 360. Rows
// 0..21 -> y offset of 32 px (HUD) + row*20 -> bottom edge at 32 + 440 = 472.
// HUD owns 0..31, bottom 8 px are just border.
const COLS = 18;
const ROWS = 22;
const CELL = 20;
const BOARD_OX = 0;
const BOARD_OY = 32;
const PLAYER_ROWS = 6;          // player can roam the bottom 6 rows

// ---- waves -------------------------------------------------------------
// Each wave: centipedeLen segments, speed in cells/sec, mushroom density.
const WAVES = [
  { name: ['Sprouts',   '萌芽'], len: 8,  speed: 3.5, mushrooms: 0.06 },
  { name: ['Garden',    '苗圃'], len: 9,  speed: 4.0, mushrooms: 0.07 },
  { name: ['Bramble',   '荆棘'], len: 10, speed: 4.6, mushrooms: 0.08 },
  { name: ['Thicket',   '密林'], len: 11, speed: 5.2, mushrooms: 0.09 },
  { name: ['Overgrowth','蔓野'], len: 12, speed: 5.9, mushrooms: 0.10 },
  { name: ['Wildwood',  '荒丛'], len: 14, speed: 6.6, mushrooms: 0.12 },
];
const WAVE_COUNT = WAVES.length;

// ---- helpers -----------------------------------------------------------
function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () { s = (s * 16807) % 2147483647; return (s - 1) / 2147483646; };
}

function makeMushroomField(density, rng) {
  // 2D grid of mushroom HP (0..3). 0 = no mushroom. The top row and the
  // bottom player area start empty; mushrooms slow movement when shot.
  const field = Array.from({ length: ROWS }, () => new Array(COLS).fill(0));
  for (let y = 2; y < ROWS - PLAYER_ROWS; y++) {
    for (let x = 0; x < COLS; x++) {
      if (rng() < density) field[y][x] = 3;
    }
  }
  return field;
}

// ---- game state --------------------------------------------------------
function buildGame(waveIndex) {
  const w = WAVES[waveIndex];
  const rng = seededRandom(waveIndex * 53 + 7);
  // The centipede starts as one worm of `w.len` segments, all at row 0,
  // moving right. Each segment is { col, row, dir: +1/-1, dropping: 0|1 }.
  const segments = [];
  for (let i = 0; i < w.len; i++) {
    segments.push({ col: i, row: 0, dir: 1, dropping: 0, age: 0 });
  }
  return {
    waveIndex, cfg: w,
    mushrooms: makeMushroomField(w.mushrooms, rng),
    segments,
    player: { col: 8.5, row: ROWS - 1, fireCd: 0, hitFlash: 0 },
    bullets: [],
    spider: null,                // {x, y, vx, vy} when active
    spiderTimer: 6 + rng() * 4,  // seconds until next spider appears
    lives: 3,
    score: 0,
    waveT: 0,
    moveAcc: 0,                  // cells of horizontal progress accumulated
    over: false, won: false,
    flash: 0,
    rng,
  };
}

// ---- input + tick ------------------------------------------------------
function setPlayerInput(s, vx, vy) {
  s.playerVX = vx; s.playerVY = vy;
}

function tick(s, dt) {
  if (s.over) return;
  s.waveT += dt;
  s.flash = Math.max(0, s.flash - dt);
  if (s.player.hitFlash > 0) s.player.hitFlash = Math.max(0, s.player.hitFlash - dt);
  // Player movement
  const ps = 7.0;                            // cells/sec
  if (s.playerVX) s.player.col = clampCol(s.player.col + s.playerVX * ps * dt);
  if (s.playerVY) s.player.row = clampRow(s.player.row + s.playerVY * ps * dt);
  // Autofire
  s.player.fireCd -= dt;
  if (s.player.fireCd <= 0) {
    s.bullets.push({ col: s.player.col, row: s.player.row - 0.6, dead: false });
    s.player.fireCd = 0.22;
  }
  // Bullets travel up at ~28 cells/sec.
  for (const b of s.bullets) {
    b.row -= 28 * dt;
    if (b.row < -0.6) b.dead = true;
  }
  // Bullet vs mushroom + bullet vs centipede.
  for (const b of s.bullets) {
    if (b.dead) continue;
    const cx = Math.round(b.col), cy = Math.round(b.row);
    if (cy < 0 || cy >= ROWS || cx < 0 || cx >= COLS) continue;
    if (s.mushrooms[cy][cx] > 0) {
      s.mushrooms[cy][cx]--;
      b.dead = true;
      if (s.mushrooms[cy][cx] === 0) s.score += 1;
      continue;
    }
    // Segment hit
    for (let i = 0; i < s.segments.length; i++) {
      const seg = s.segments[i];
      if (seg.col === cx && seg.row === cy) {
        // Segment dies; mushroom takes its place; the segment's neighbours
        // become heads of two split worms (we just keep the segment array
        // ordered and let the chase logic re-evaluate "dir" on next bump).
        s.mushrooms[cy][cx] = 3;
        s.segments.splice(i, 1);
        s.score += 10;
        b.dead = true;
        break;
      }
    }
  }
  s.bullets = s.bullets.filter(b => !b.dead);
  // Centipede step
  s.moveAcc += s.cfg.speed * dt;
  while (s.moveAcc >= 1 && !s.over) {
    s.moveAcc -= 1;
    stepCentipede(s);
  }
  // Wave clear — the last segment may have been shot off this tick, before
  // any centipede step ran, so check unconditionally.
  if (!s.over && s.segments.length === 0) {
    s.won = true; s.over = true;
    s.flash = 0.5;
    s.score += 100;
    return;
  }
  // Spider
  s.spiderTimer -= dt;
  if (!s.spider && s.spiderTimer <= 0) {
    const fromLeft = s.rng() < 0.5;
    s.spider = {
      col: fromLeft ? -0.5 : COLS - 0.5,
      row: ROWS - 3 + s.rng() * 2,
      vx: (fromLeft ? 1 : -1) * (4 + s.rng() * 2),
      vy: (s.rng() < 0.5 ? -1 : 1) * 2.4,
      anim: 0,
    };
  }
  if (s.spider) {
    const sp = s.spider;
    sp.col += sp.vx * dt;
    sp.row += sp.vy * dt;
    sp.anim += dt;
    if (sp.row < ROWS - PLAYER_ROWS + 0.5 || sp.row > ROWS - 0.5) sp.vy *= -1;
    if (sp.col < -1 || sp.col > COLS + 0.5) { s.spider = null; s.spiderTimer = 5 + s.rng() * 5; }
    else {
      // Bullet vs spider
      for (const b of s.bullets) {
        if (!b.dead && Math.abs(b.col - sp.col) < 0.6 && Math.abs(b.row - sp.row) < 0.6) {
          const d = Math.abs(b.row - s.player.row);
          s.score += d < 2 ? 600 : d < 4 ? 300 : 200;
          s.spider = null; s.spiderTimer = 6 + s.rng() * 4;
          b.dead = true;
          break;
        }
      }
      // Spider vs player
      if (s.spider && Math.abs(sp.col - s.player.col) < 0.7 && Math.abs(sp.row - s.player.row) < 0.7) {
        die(s);
        s.spider = null;
        s.spiderTimer = 5;
      }
    }
  }
  // Centipede vs player
  if (!s.over) {
    for (const seg of s.segments) {
      if (Math.abs(seg.col - s.player.col) < 0.7 && Math.abs(seg.row - s.player.row) < 0.7) {
        die(s);
        break;
      }
    }
  }
}

// One discrete step of the centipede. Each segment moves one cell in its
// dir; if it hits a wall or a mushroom, it drops a row and reverses.
function stepCentipede(s) {
  for (const seg of s.segments) {
    if (seg.dropping > 0) {
      seg.row += 1;
      seg.dropping = 0;
      seg.dir = -seg.dir;
      seg.col = clampCol(seg.col);
      if (seg.row >= ROWS) seg.row = ROWS - 1;
      continue;
    }
    const nx = seg.col + seg.dir;
    let block = false;
    if (nx < 0 || nx >= COLS) block = true;
    else if (s.mushrooms[seg.row][nx] > 0) block = true;
    if (block) {
      seg.dropping = 1;
      seg.row += 1;
      seg.dir = -seg.dir;
      seg.col = clampCol(seg.col);
      if (seg.row >= ROWS) seg.row = ROWS - 1;
    } else {
      seg.col = nx;
    }
    seg.age++;
  }
}

function die(s) {
  s.lives--;
  s.player.hitFlash = 0.6;
  s.flash = 0.4;
  s.player.col = COLS / 2;
  s.player.row = ROWS - 1;
  if (s.lives < 0) { s.over = true; s.won = false; }
}

function clampCol(c) { return Math.max(0, Math.min(COLS - 1, c)); }
function clampRow(r) {
  const minRow = ROWS - PLAYER_ROWS;
  return Math.max(minRow, Math.min(ROWS - 1, r));
}

function finalScore(s) {
  return s.score + Math.max(0, s.lives) * 50;
}
