// Pixel Leap - a single-screen precision platformer.
//
// Run, jump and air-dash through a hand-built room of ledges and spikes,
// gather every gem, and reach the exit. Touch a spike or fall out the
// bottom and you respawn at once - the deaths counter is your only cost.

const VW = 360, VH = 480;

const TILE = 24, COLS = 15, ROWS = 15;
const OX = 0, OY = 36;                     // play area origin (HUD strip on top)

// Tile chars: '#' solid, '^' spike, 'S' spawn, 'E' exit, 'o' gem, ' ' empty.
// The hero walks row 10 on the row-11 floor; gaps in that floor are the
// challenge - some bottom out on solid (a shallow pit), others on spikes.
const LEVELS = [
  { name: ['First Steps', '起步'], rows: [
    '###############',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#......o......#',
    '#.............#',
    '#.S.........E.#',
    '###############',
    '###############',
    '###############',
    '###############',
  ] },
  { name: ['First Gaps', '初隙'], rows: [
    '###############',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#......o......#',
    '#.............#',
    '#.S.........E.#',
    '####..####..###',
    '###############',
    '###############',
    '###############',
  ] },
  { name: ['Deadly Drop', '致命落'], rows: [
    '###############',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#......o......#',
    '#.............#',
    '#.S.........E.#',
    '####..####..###',
    '##########^^###',
    '###############',
    '###############',
  ] },
  { name: ['The Gap', '深渊'], rows: [
    '###############',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#......o......#',
    '#.............#',
    '#.S.........E.#',
    '#####..########',
    '#####^^########',
    '###############',
    '###############',
  ] },
  { name: ['Twin Pits', '双坑'], rows: [
    '###############',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.......o.....#',
    '#.............#',
    '#.S.........E.#',
    '####..##..#####',
    '########^^#####',
    '###############',
    '###############',
  ] },
  { name: ['Gauntlet', '试炼'], rows: [
    '###############',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#.............#',
    '#......o......#',
    '#.............#',
    '#.S.........E.#',
    '####..####..###',
    '####^^####^^###',
    '###############',
    '###############',
  ] },
];
const LEVEL_COUNT = LEVELS.length;

// ---- physics constants -------------------------------------------------
const HERO_W = 14, HERO_H = 20;
const GRAVITY = 1500, MAX_FALL = 640;
const RUN_SPEED = 150, RUN_ACCEL = 1500;
const JUMP_V = 462, JUMP_CUT = 168;
const COYOTE = 0.085, JUMP_BUFFER = 0.10;
const DASH_SPEED = 332, DASH_TIME = 0.15;

function buildGame(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const grid = [];
  let sx = 1, sy = 1;
  const gems = [];
  let exit = { c: 1, r: 1 };
  for (let r = 0; r < ROWS; r++) {
    const row = [];
    for (let c = 0; c < COLS; c++) {
      let ch = (cfg.rows[r] && cfg.rows[r][c]) || '#';
      if (ch === 'S') { sx = c; sy = r; ch = ' '; }
      else if (ch === 'E') { exit = { c, r }; ch = ' '; }
      else if (ch === 'o') { gems.push({ c, r, got: false }); ch = ' '; }
      row.push(ch);
    }
    grid.push(row);
  }
  return {
    levelIndex, cfg, grid, gems, exit,
    spawn: { x: OX + sx * TILE + (TILE - HERO_W) / 2, y: OY + sy * TILE + TILE - HERO_H },
    hero: null,
    inLeft: false, inRight: false, jumpHeld: false,
    facing: 1,
    deaths: 0,
    over: false, won: false,
  };
}

function resetHero(s) {
  s.hero = {
    x: s.spawn.x, y: s.spawn.y, vx: 0, vy: 0,
    onGround: false, coyote: 0, jumpBuf: 0,
    dashAvail: true, dashT: 0,
  };
}

// ---- tile helpers ------------------------------------------------------
function solidAt(s, c, r) {
  if (c < 0 || c >= COLS) return true;          // side walls
  if (r < 0) return true;                       // ceiling
  if (r >= ROWS) return false;                  // open below = a pit
  return s.grid[r][c] === '#';
}

// Any spike tile overlapping the box [x,y,w,h]?
function spikeHit(s, x, y, w, h) {
  const c0 = Math.floor((x - OX) / TILE), c1 = Math.floor((x + w - 1 - OX) / TILE);
  const r0 = Math.floor((y - OY) / TILE), r1 = Math.floor((y + h - 1 - OY) / TILE);
  for (let r = r0; r <= r1; r++) {
    for (let c = c0; c <= c1; c++) {
      if (r >= 0 && r < ROWS && c >= 0 && c < COLS && s.grid[r][c] === '^') return true;
    }
  }
  return false;
}

function boxHitsSolid(s, x, y, w, h) {
  const c0 = Math.floor((x - OX) / TILE), c1 = Math.floor((x + w - 1 - OX) / TILE);
  const r0 = Math.floor((y - OY) / TILE), r1 = Math.floor((y + h - 1 - OY) / TILE);
  for (let r = r0; r <= r1; r++)
    for (let c = c0; c <= c1; c++)
      if (solidAt(s, c, r)) return true;
  return false;
}

// Solid tile directly under the hero's feet? A stable probe, so a hero
// resting on a ledge stays "grounded" instead of micro-bouncing.
function onGroundCheck(s, hx, hy) {
  const r = Math.floor((hy + HERO_H + 1 - OY) / TILE);
  const c0 = Math.floor((hx - OX) / TILE), c1 = Math.floor((hx + HERO_W - 1 - OX) / TILE);
  for (let c = c0; c <= c1; c++) if (solidAt(s, c, r)) return true;
  return false;
}

// ---- input -------------------------------------------------------------
function setMove(s, dir) {
  s.inLeft = dir < 0;
  s.inRight = dir > 0;
  if (dir !== 0) s.facing = dir;
}
function jump(s) {
  if (s.over || !s.hero) return;
  s.hero.jumpBuf = JUMP_BUFFER;
}
function dash(s) {
  if (s.over || !s.hero) return;
  const h = s.hero;
  if (h.dashAvail && h.dashT <= 0) {
    h.dashT = DASH_TIME;
    h.dashAvail = false;
    h.vx = s.facing * DASH_SPEED;
    h.vy = 0;
  }
}

// ---- tick --------------------------------------------------------------
function tick(s, dt) {
  if (s.over || !s.hero) return;
  let remain = Math.min(dt, 0.05);
  while (remain > 0) {
    const step = Math.min(1 / 240, remain);
    substep(s, step);
    remain -= step;
    if (s.over) return;
  }
}

function substep(s, dt) {
  const h = s.hero;
  // Timers.
  if (h.coyote > 0) h.coyote = Math.max(0, h.coyote - dt);
  if (h.jumpBuf > 0) h.jumpBuf = Math.max(0, h.jumpBuf - dt);
  if (h.dashT > 0) h.dashT = Math.max(0, h.dashT - dt);
  const dashing = h.dashT > 0;

  // Jump (buffered) - usable on ground or within coyote time.
  if (h.jumpBuf > 0 && (h.onGround || h.coyote > 0)) {
    h.vy = -JUMP_V;
    h.onGround = false;
    h.coyote = 0;
    h.jumpBuf = 0;
  }
  // Variable jump height.
  if (!s.jumpHeld && h.vy < -JUMP_CUT && !dashing) h.vy = -JUMP_CUT;

  if (dashing) {
    h.vx = s.facing * DASH_SPEED;
    h.vy = 0;
  } else {
    const target = (s.inRight ? 1 : 0) - (s.inLeft ? 1 : 0);
    const goal = target * RUN_SPEED;
    if (h.vx < goal) h.vx = Math.min(goal, h.vx + RUN_ACCEL * dt);
    else if (h.vx > goal) h.vx = Math.max(goal, h.vx - RUN_ACCEL * dt);
    h.vy = Math.min(MAX_FALL, h.vy + GRAVITY * dt);
  }

  // Move X, resolve.
  h.x += h.vx * dt;
  if (boxHitsSolid(s, h.x, h.y, HERO_W, HERO_H)) {
    const dir = h.vx > 0 ? 1 : -1;
    while (boxHitsSolid(s, h.x, h.y, HERO_W, HERO_H)) h.x -= dir;
    h.vx = 0;
    if (dashing) h.dashT = 0;
  }
  // Move Y, resolve hard overlaps (ceiling / floor).
  h.y += h.vy * dt;
  if (boxHitsSolid(s, h.x, h.y, HERO_W, HERO_H)) {
    const dir = h.vy > 0 ? 1 : -1;
    while (boxHitsSolid(s, h.x, h.y, HERO_W, HERO_H)) h.y -= dir;
    h.vy = 0;
  }
  // Ground via a stable probe - resting on a ledge never flickers.
  h.onGround = onGroundCheck(s, h.x, h.y);
  if (h.onGround && h.vy > 0) h.vy = 0;
  if (h.onGround) { h.coyote = COYOTE; h.dashAvail = true; }

  // Hazards.
  if (spikeHit(s, h.x, h.y, HERO_W, HERO_H)) { die(s); return; }
  if (h.y > OY + ROWS * TILE + 30) { die(s); return; }

  // Gems.
  for (const g of s.gems) {
    if (g.got) continue;
    const gx = OX + g.c * TILE, gy = OY + g.r * TILE;
    if (h.x < gx + TILE && h.x + HERO_W > gx && h.y < gy + TILE && h.y + HERO_H > gy) g.got = true;
  }
  // Exit - reach it to clear the room (gems are an optional bonus).
  {
    const ex = OX + s.exit.c * TILE, ey = OY + s.exit.r * TILE;
    if (h.x < ex + TILE && h.x + HERO_W > ex && h.y < ey + TILE && h.y + HERO_H > ey) {
      s.over = true; s.won = true;
    }
  }
}

function die(s) {
  s.deaths++;
  resetHero(s);
}

function gemsLeft(s) { return s.gems.filter(g => !g.got).length; }
