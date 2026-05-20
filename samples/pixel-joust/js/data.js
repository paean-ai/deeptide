// Pixel Joust - flap-fly lance combat in the spirit of Williams' Joust.
// Tap to flap; gravity pulls down; the higher lance wins on collision.
//
// The arena wraps horizontally (touch the left wall, reappear on the
// right) and has a fixed top + ground. A handful of horizontal platforms
// give riders perches between flaps.

const VW = 360, VH = 480;
const ARENA_TOP = 32;
const ARENA_BOTTOM = 440;       // ground floor y
const ARENA_HEIGHT = ARENA_BOTTOM - ARENA_TOP;
const GRAVITY = 720;
const FLAP_IMPULSE = -240;
const MOVE_SPEED = 130;         // horizontal cells/sec when input held
const RIDER_W = 22;
const RIDER_H = 22;
const LANCE_TIP = 6;            // pixels the lance pokes up above the rider top
const COL_DX = 18;              // collision: horizontal distance threshold
const COL_DY = 18;              // collision: vertical distance threshold
const LANCE_GAP = 3;            // pixels of lance-height advantage required for a kill

// Three horizontal platforms (top, mid, lower) for short rests.
const PLATFORMS = [
  { x:  60, y: 144, w: 100 },
  { x: 200, y: 220, w: 100 },
  { x:  40, y: 320, w:  80 },
  { x: 240, y: 320, w:  80 },
];

// ---- waves -------------------------------------------------------------
const LEVELS = [
  { name: ['Squires',    '随侍'], enemies: 2, speed: 1.0 },
  { name: ['Footmen',    '步卒'], enemies: 3, speed: 1.1 },
  { name: ['Knights',    '骑士'], enemies: 4, speed: 1.2 },
  { name: ['Champions',  '勇士'], enemies: 5, speed: 1.35 },
  { name: ['Crusaders',  '征士'], enemies: 6, speed: 1.55 },
  { name: ['Black Lance','黑骑士'], enemies: 7, speed: 1.8 },
];
const LEVEL_COUNT = LEVELS.length;

// ---- helpers -----------------------------------------------------------
function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () { s = (s * 16807) % 2147483647; return (s - 1) / 2147483646; };
}

// Wrap an x position into the arena. Negative x wraps to right; > VW to left.
function wrapX(x) {
  if (x < 0)  return x + VW;
  if (x >= VW) return x - VW;
  return x;
}

// Lance "tip" Y for a rider — riders facing each other determine who wins
// by which lance tip is HIGHER (smaller y). A small LANCE_GAP keeps near-
// ties from becoming kills both ways.
function lanceY(rider) { return rider.y - LANCE_TIP; }

// Resolve a collision between attacker (a) and defender (b). Returns:
//   'a_wins' if a's lance is sufficiently higher,
//   'b_wins' if b's lance is sufficiently higher,
//   'bump' if both are within LANCE_GAP (just bounce).
function resolveJoust(a, b) {
  const la = lanceY(a), lb = lanceY(b);
  if (la + LANCE_GAP < lb) return 'a_wins';
  if (lb + LANCE_GAP < la) return 'b_wins';
  return 'bump';
}

// ---- runtime state -----------------------------------------------------
function buildGame(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const rng = seededRandom(levelIndex * 37 + 11);
  return {
    levelIndex, cfg, rng,
    player: {
      x: VW / 2, y: 360, vx: 0, vy: 0,
      face: 1, alive: true, hitFlash: 0, respawn: 0, eggsPicked: 0,
    },
    enemies: spawnEnemies(cfg, rng),
    eggs: [],
    input: { left: false, right: false, flap: false },
    flapQueued: false,
    score: 0,
    lives: 2,
    over: false, won: false,
    flash: 0,
  };
}

function spawnEnemies(cfg, rng) {
  const list = [];
  for (let i = 0; i < cfg.enemies; i++) {
    const left = rng() < 0.5;
    list.push({
      x: left ? 30 : VW - 30,
      y: ARENA_TOP + 30 + rng() * 60,
      vx: (left ? 1 : -1) * (60 + rng() * 60) * cfg.speed,
      vy: 0,
      flapT: 0.4 + rng() * 0.5,
      face: left ? 1 : -1,
      alive: true,
      color: '#ff7a7a',
    });
  }
  return list;
}

// ---- input -------------------------------------------------------------
function setInput(s, key, on) {
  if (s.over) return;
  if (key === 'left' || key === 'right') s.input[key] = on;
  else if (key === 'flap' && on) s.flapQueued = true;
}

// ---- tick --------------------------------------------------------------
function tick(s, dt) {
  if (s.over) return;
  s.flash = Math.max(0, s.flash - dt);
  // Respawn beat.
  if (!s.player.alive) {
    s.player.respawn -= dt;
    if (s.player.respawn <= 0) resetPlayer(s);
    advanceEnemies(s, dt);
    return;
  }
  // Player physics.
  const p = s.player;
  if (p.hitFlash > 0) p.hitFlash = Math.max(0, p.hitFlash - dt);
  // Input -> velocity.
  if (s.input.left)  { p.vx = -MOVE_SPEED; p.face = -1; }
  else if (s.input.right) { p.vx = MOVE_SPEED; p.face = 1; }
  else p.vx *= Math.pow(0.5, dt * 4);
  if (s.flapQueued) { p.vy = FLAP_IMPULSE; s.flapQueued = false; s.score += 1; }
  p.vy += GRAVITY * dt;
  p.x += p.vx * dt; p.y += p.vy * dt;
  // Wrap horizontally.
  p.x = wrapX(p.x);
  // Ground = death (player falls into the floor).
  if (p.y + RIDER_H / 2 >= ARENA_BOTTOM) { die(s); return; }
  // Ceiling cap.
  if (p.y < ARENA_TOP + 10) { p.y = ARENA_TOP + 10; p.vy = Math.max(0, p.vy); }
  // Platform support: if dropping, land on a platform top.
  if (p.vy > 0) {
    for (const pl of PLATFORMS) {
      if (p.x > pl.x - RIDER_W / 2 && p.x < pl.x + pl.w + RIDER_W / 2 &&
          p.y + RIDER_H / 2 >= pl.y && p.y + RIDER_H / 2 < pl.y + 8) {
        p.y = pl.y - RIDER_H / 2;
        p.vy = 0;
        break;
      }
    }
  }
  advanceEnemies(s, dt);
  resolveCollisions(s);
  // Egg pickup.
  for (const e of s.eggs) {
    if (e.dead) continue;
    if (Math.abs(e.x - p.x) < 14 && Math.abs(e.y - p.y) < 14) {
      e.dead = true; p.eggsPicked++; s.score += 25; s.flash = 0.2;
    }
    // Eggs eventually hatch into a new enemy.
    e.timer -= dt;
    if (e.timer <= 0) {
      e.dead = true;
      s.enemies.push({
        x: e.x, y: e.y - 12, vx: (s.rng() < 0.5 ? -1 : 1) * 70 * s.cfg.speed,
        vy: 0, flapT: 0.4, face: 1, alive: true, color: '#ff9b9b',
      });
    }
  }
  s.eggs = s.eggs.filter(e => !e.dead);
  // Wave clear: all enemies defeated, no eggs left to hatch.
  if (s.enemies.every(e => !e.alive) && s.eggs.length === 0) {
    s.over = true; s.won = true; s.score += 300; s.flash = 0.55;
  }
}

function advanceEnemies(s, dt) {
  for (const e of s.enemies) {
    if (!e.alive) continue;
    e.flapT -= dt;
    if (e.flapT <= 0) {
      e.vy = FLAP_IMPULSE * (0.7 + s.rng() * 0.4);
      e.flapT = 0.6 + s.rng() * 0.6;
    }
    e.vy += GRAVITY * dt;
    e.x += e.vx * dt; e.y += e.vy * dt;
    e.x = wrapX(e.x);
    e.face = e.vx >= 0 ? 1 : -1;
    if (e.y < ARENA_TOP + 10) { e.y = ARENA_TOP + 10; e.vy = Math.max(0, e.vy); }
    if (e.y + RIDER_H / 2 >= ARENA_BOTTOM - 1) {
      // Enemy hitting the ground also dies (drops an egg).
      enemyDie(s, e);
    }
    for (const pl of PLATFORMS) {
      if (e.vy > 0 && e.x > pl.x - RIDER_W / 2 && e.x < pl.x + pl.w + RIDER_W / 2 &&
          e.y + RIDER_H / 2 >= pl.y && e.y + RIDER_H / 2 < pl.y + 8) {
        e.y = pl.y - RIDER_H / 2; e.vy = 0;
      }
    }
  }
}

function resolveCollisions(s) {
  const p = s.player;
  if (!p.alive) return;
  for (const e of s.enemies) {
    if (!e.alive) continue;
    if (Math.abs(e.x - p.x) < COL_DX && Math.abs(e.y - p.y) < COL_DY) {
      const r = resolveJoust(p, e);
      if (r === 'a_wins') {
        enemyDie(s, e);
        // Tiny knockback on the player.
        p.vx = -p.face * 60;
      } else if (r === 'b_wins') {
        die(s); return;
      } else {
        // Bump: separate them.
        const dx = (p.x - e.x) || (p.face);
        p.vx = Math.sign(dx) * 90;
        e.vx = -Math.sign(dx) * 90;
      }
    }
  }
}

function enemyDie(s, e) {
  e.alive = false;
  s.score += 100;
  s.flash = 0.25;
  // Drop an egg at the enemy's position; eggs fall to nearest platform / floor.
  s.eggs.push({ x: e.x, y: e.y, dead: false, timer: 4.5 });
}

function die(s) {
  const p = s.player;
  if (!p.alive) return;
  p.alive = false;
  p.hitFlash = 0.6;
  s.flash = 0.4;
  s.lives--;
  if (s.lives < 0) { s.over = true; s.won = false; return; }
  p.respawn = 0.7;
}

function resetPlayer(s) {
  const p = s.player;
  p.x = VW / 2; p.y = 360;
  p.vx = 0; p.vy = 0;
  p.face = 1;
  p.alive = true;
}

function finalScore(s) {
  return s.score + Math.max(0, s.lives) * 100;
}
