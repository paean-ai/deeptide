// Pixel Vector - an Asteroids-style space shooter. Drift, rotate, thrust, fire;
// blast asteroids until the field clears. Edges wrap.

const VW = 360, VH = 480;
const FIELD_TOP = 60, FIELD_BOTTOM = 388;       // play area below HUD, above controls
const FIELD_H = FIELD_BOTTOM - FIELD_TOP;        // 328
const FIELD_W = VW;

const ROT_SPEED = 4.2;     // rad/s
const THRUST = 220;        // px/s^2
const DRAG = 0.45;         // per second
const MAX_SPEED = 240;
const BULLET_SPEED = 360;
const BULLET_LIFE = 0.95;
const FIRE_CD = 0.22;
const SHIP_R = 8;
const RESPAWN_INVULN = 1.5;
const RESPAWN_DELAY = 0.8;

const A_SPEED = [0, 60, 80, 100];                // by size 1..3
const A_RADIUS = [0, 8, 14, 22];
const A_SCORE = [0, 40, 20, 10];

const LEVELS = [
  { name: ['Drift', '漂流'],     seed: 17,  big: 3, med: 0 },
  { name: ['Belt', '小行星带'],  seed: 53,  big: 4, med: 0 },
  { name: ['Stream', '陨流'],    seed: 109, big: 5, med: 0 },
  { name: ['Storm', '陨石风暴'], seed: 184, big: 4, med: 2 },
  { name: ['Cluster', '簇群'],   seed: 268, big: 6, med: 0 },
  { name: ['Maelstrom', '漩涡'], seed: 362, big: 5, med: 3 },
];
const LEVEL_COUNT = LEVELS.length;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () {
    s = (s * 16807) % 2147483647;
    return (s - 1) / 2147483646;
  };
}

function wrapPos(p) {
  if (p.x < 0) p.x += FIELD_W; else if (p.x >= FIELD_W) p.x -= FIELD_W;
  if (p.y < FIELD_TOP) p.y += FIELD_H; else if (p.y >= FIELD_BOTTOM) p.y -= FIELD_H;
}
function wrapDelta(a, b) {
  // shortest delta a->b on the toroidal field
  let dx = b.x - a.x, dy = b.y - a.y;
  if (dx > FIELD_W / 2) dx -= FIELD_W; else if (dx < -FIELD_W / 2) dx += FIELD_W;
  if (dy > FIELD_H / 2) dy -= FIELD_H; else if (dy < -FIELD_H / 2) dy += FIELD_H;
  return { dx, dy };
}

function makeAsteroid(s, x, y, size, vx, vy) {
  const verts = [];
  const n = 10;
  for (let i = 0; i < n; i++) {
    const a = (i / n) * Math.PI * 2;
    const r = A_RADIUS[size] * (0.78 + s.rng() * 0.36);
    verts.push({ x: Math.cos(a) * r, y: Math.sin(a) * r });
  }
  return { x, y, vx, vy, size, r: A_RADIUS[size], verts, rot: s.rng() * Math.PI * 2, spin: (s.rng() - 0.5) * 0.6 };
}

function spawnEdgeAsteroid(s, size) {
  // pick a spawn on an edge of the field, away from ship
  for (let tries = 0; tries < 20; tries++) {
    const side = (s.rng() * 4) | 0;
    let x, y;
    if (side === 0) { x = s.rng() * FIELD_W; y = FIELD_TOP + 4; }
    else if (side === 1) { x = FIELD_W - 4; y = FIELD_TOP + s.rng() * FIELD_H; }
    else if (side === 2) { x = s.rng() * FIELD_W; y = FIELD_BOTTOM - 4; }
    else { x = 4; y = FIELD_TOP + s.rng() * FIELD_H; }
    if (s.ship && Math.hypot(x - s.ship.x, y - (s.ship.y)) < 80) continue;
    const a = s.rng() * Math.PI * 2;
    const v = A_SPEED[size] * (0.7 + s.rng() * 0.4);
    return makeAsteroid(s, x, y, size, Math.cos(a) * v, Math.sin(a) * v);
  }
  const a = s.rng() * Math.PI * 2;
  const v = A_SPEED[size];
  return makeAsteroid(s, FIELD_W / 2, FIELD_TOP + FIELD_H / 2, size, Math.cos(a) * v, Math.sin(a) * v);
}

function buildGame(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const s = {
    levelIndex, cfg,
    rng: seededRandom(cfg.seed),
    ship: null, lives: 3, score: 0,
    asteroids: [], bullets: [],
    won: false, over: false,
    fireCd: 0, invuln: 0, respawnT: 0,
  };
  s.ship = { x: FIELD_W / 2, y: FIELD_TOP + FIELD_H / 2, vx: 0, vy: 0, angle: -Math.PI / 2, alive: true };
  for (let i = 0; i < cfg.big; i++) s.asteroids.push(spawnEdgeAsteroid(s, 3));
  for (let i = 0; i < (cfg.med || 0); i++) s.asteroids.push(spawnEdgeAsteroid(s, 2));
  s.invuln = RESPAWN_INVULN;
  return s;
}

// ---- input commands -----------------------------------------------------
function fire(s) {
  if (s.over || !s.ship.alive || s.fireCd > 0) return false;
  s.bullets.push({
    x: s.ship.x + Math.cos(s.ship.angle) * SHIP_R,
    y: s.ship.y + Math.sin(s.ship.angle) * SHIP_R,
    vx: Math.cos(s.ship.angle) * BULLET_SPEED + s.ship.vx,
    vy: Math.sin(s.ship.angle) * BULLET_SPEED + s.ship.vy,
    life: BULLET_LIFE,
  });
  s.fireCd = FIRE_CD;
  return true;
}

// ---- world tick ----------------------------------------------------------
function tick(s, dt, input) {
  if (s.over) return;
  if (!s.ship.alive) {
    s.respawnT -= dt;
    if (s.respawnT <= 0 && s.lives > 0) {
      s.ship = { x: FIELD_W / 2, y: FIELD_TOP + FIELD_H / 2, vx: 0, vy: 0, angle: -Math.PI / 2, alive: true };
      s.invuln = RESPAWN_INVULN;
    }
  } else {
    if (input) {
      if (input.left)  s.ship.angle -= ROT_SPEED * dt;
      if (input.right) s.ship.angle += ROT_SPEED * dt;
      if (input.thrust) {
        s.ship.vx += Math.cos(s.ship.angle) * THRUST * dt;
        s.ship.vy += Math.sin(s.ship.angle) * THRUST * dt;
        const sp = Math.hypot(s.ship.vx, s.ship.vy);
        if (sp > MAX_SPEED) { s.ship.vx *= MAX_SPEED / sp; s.ship.vy *= MAX_SPEED / sp; }
      }
      if (input.fire) fire(s);
    }
    s.ship.vx *= (1 - DRAG * dt);
    s.ship.vy *= (1 - DRAG * dt);
    s.ship.x += s.ship.vx * dt;
    s.ship.y += s.ship.vy * dt;
    wrapPos(s.ship);
    if (s.invuln > 0) s.invuln -= dt;
  }
  s.fireCd = Math.max(0, s.fireCd - dt);
  // bullets
  for (const b of s.bullets) {
    b.x += b.vx * dt; b.y += b.vy * dt; b.life -= dt;
    wrapPos(b);
  }
  s.bullets = s.bullets.filter(b => b.life > 0);
  // asteroids
  for (const a of s.asteroids) {
    a.x += a.vx * dt; a.y += a.vy * dt; a.rot += a.spin * dt;
    wrapPos(a);
  }
  // bullet-asteroid collisions
  for (const b of s.bullets) {
    if (b.dead) continue;
    for (let i = 0; i < s.asteroids.length; i++) {
      const a = s.asteroids[i];
      const d = wrapDelta(a, b);
      if (d.dx * d.dx + d.dy * d.dy < a.r * a.r) {
        b.dead = true;
        s.score += A_SCORE[a.size];
        s.asteroids.splice(i, 1);
        if (a.size > 1) {
          const ang = s.rng() * Math.PI * 2;
          for (let k = 0; k < 2; k++) {
            const aa = ang + (k === 0 ? -0.6 : 0.6);
            const v = A_SPEED[a.size - 1] * (0.8 + s.rng() * 0.4);
            s.asteroids.push(makeAsteroid(s, a.x, a.y, a.size - 1, Math.cos(aa) * v, Math.sin(aa) * v));
          }
        }
        break;
      }
    }
  }
  s.bullets = s.bullets.filter(b => !b.dead);
  // asteroid-ship collisions
  if (s.ship.alive && s.invuln <= 0) {
    for (const a of s.asteroids) {
      const d = wrapDelta(a, s.ship);
      if (d.dx * d.dx + d.dy * d.dy < (a.r + SHIP_R) * (a.r + SHIP_R)) {
        killShip(s);
        break;
      }
    }
  }
  if (!s.over && s.asteroids.length === 0) { s.won = true; s.over = true; }
}

function killShip(s) {
  s.ship.alive = false;
  s.lives--;
  s.respawnT = RESPAWN_DELAY;
  if (s.lives <= 0) { s.over = true; s.won = false; }
}
