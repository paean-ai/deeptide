// Pixel Snowboard - a downhill slalom. The slope scrolls upward as the
// rider descends; carve left and right to thread the trees, slip through
// the slalom gates for bonus points, and hit the ramps to launch into a
// trick (a brief invulnerable hop). Reach the finish banner to clear the
// run. Each obstacle field is generated from a seed.

const VW = 360, VH = 480;
const RIDER_Y = 150;            // the rider's fixed screen y; the world scrolls
const RIDER_W = 20, RIDER_H = 24;
const STEER_SPEED = 200;        // px/s of lateral carve

// Obstacle kinds: 'tree' (fatal), 'rock' (fatal), 'gate' (a pair of flags —
// pass between them for bonus), 'ramp' (launches a trick hop).
const LEVELS = [
  { name: ['Bunny',    '初级道'], length: 2400, scroll: 150, density: 0.7,  seed: 11 },
  { name: ['Green',    '绿道'],   length: 3000, scroll: 175, density: 0.85, seed: 22 },
  { name: ['Blue',     '蓝道'],   length: 3600, scroll: 205, density: 1.0,  seed: 33 },
  { name: ['Black',    '黑道'],   length: 4200, scroll: 235, density: 1.2,  seed: 44 },
  { name: ['Couloir',  '雪槽'],   length: 4800, scroll: 265, density: 1.4,  seed: 55 },
  { name: ['Cornice',  '悬顶'],   length: 5400, scroll: 300, density: 1.6,  seed: 66 },
];
const LEVEL_COUNT = LEVELS.length;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () { s = (s * 16807) % 2147483647; return (s - 1) / 2147483646; };
}

// Build the obstacle field. Each obstacle has a `y` in WORLD units
// (0 at the top / start, growing downhill) and an `x` screen column.
function buildField(cfg, rng) {
  const obstacles = [];
  // Lay obstacles in rows spaced by a gap that tightens with density.
  const gap = 150 / cfg.density;
  for (let y = 320; y < cfg.length - 240; y += gap * (0.7 + rng() * 0.6)) {
    const roll = rng();
    if (roll < 0.16) {
      // Slalom gate: two flags with a passable lane between them.
      const laneX = 60 + rng() * (VW - 120);
      obstacles.push({ kind: 'gate', y, x: laneX, w: 70, scored: false });
    } else if (roll < 0.30) {
      obstacles.push({ kind: 'ramp', y, x: 40 + rng() * (VW - 80), used: false });
    } else if (roll < 0.42) {
      obstacles.push({ kind: 'rock', y, x: 30 + rng() * (VW - 60) });
    } else {
      // Trees — sometimes a small cluster.
      const cluster = 1 + ((rng() * 3) | 0);
      const baseX = 30 + rng() * (VW - 60);
      for (let k = 0; k < cluster; k++) {
        obstacles.push({ kind: 'tree', y: y + k * 22, x: baseX + (rng() - 0.5) * 60 });
      }
    }
  }
  return obstacles;
}

// ---- runtime state -----------------------------------------------------
function buildGame(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const rng = seededRandom(cfg.seed);
  return {
    levelIndex, cfg,
    obstacles: buildField(cfg, rng),
    rider: { x: VW / 2, air: 0, hitFlash: 0, alive: true },
    worldY: 0,                  // how far down the slope we've travelled
    scroll: cfg.scroll,
    inputX: null,
    score: 0,
    lives: 2,
    over: false, won: false,
    flash: 0,
    trickT: 0,
  };
}

// ---- input -------------------------------------------------------------
function setRiderX(s, x) {
  if (s.over) return;
  s.inputX = Math.max(RIDER_W / 2, Math.min(VW - RIDER_W / 2, x));
}

// ---- tick --------------------------------------------------------------
function tick(s, dt) {
  if (s.over) return;
  s.flash = Math.max(0, s.flash - dt);
  if (s.trickT > 0) s.trickT = Math.max(0, s.trickT - dt);
  const r = s.rider;
  if (r.hitFlash > 0) r.hitFlash = Math.max(0, r.hitFlash - dt);
  if (!r.alive) {
    r.respawn = (r.respawn || 0) - dt;
    if (r.respawn <= 0) { r.alive = true; r.hitFlash = 0.8; r.x = VW / 2; r.air = 0; }
    return;
  }
  // Carve toward the input column.
  if (s.inputX !== null) {
    const dx = s.inputX - r.x;
    r.x += Math.sign(dx) * Math.min(Math.abs(dx), STEER_SPEED * dt);
  }
  // Air timer (mid-trick = invulnerable).
  if (r.air > 0) r.air = Math.max(0, r.air - dt);
  // Descend.
  s.worldY += s.scroll * dt;
  s.score += s.scroll * dt * 0.02;
  // Finish line.
  if (s.worldY >= s.cfg.length) {
    s.over = true; s.won = true;
    s.score += 500 + s.lives * 100;
    s.flash = 0.55;
    return;
  }
  // Collisions: an obstacle is "at the rider" when its screen y (worldY
  // offset) is within the rider band.
  for (const o of s.obstacles) {
    const screenY = RIDER_Y + (o.y - s.worldY);
    if (screenY < RIDER_Y - 30 || screenY > RIDER_Y + 30) continue;
    if (o.kind === 'gate') {
      // Score once when the rider's column is inside the gate lane.
      if (!o.scored && Math.abs(screenY - RIDER_Y) < 14) {
        const inLane = r.x > o.x - o.w / 2 && r.x < o.x + o.w / 2;
        o.scored = true;
        if (inLane) { s.score += 120; s.flash = 0.2; }
      }
    } else if (o.kind === 'ramp') {
      if (!o.used && Math.abs(screenY - RIDER_Y) < 14 &&
          Math.abs(r.x - o.x) < 26) {
        o.used = true;
        r.air = 0.9;            // 0.9 s of trick airtime + invuln
        s.trickT = 0.9;
        s.score += 60;
        s.flash = 0.2;
      }
    } else {
      // tree / rock — fatal unless the rider is airborne.
      if (r.air <= 0 && Math.abs(screenY - RIDER_Y) < 16 &&
          Math.abs(r.x - o.x) < (RIDER_W / 2 + 10)) {
        crash(s);
        return;
      }
    }
  }
}

function crash(s) {
  const r = s.rider;
  if (!r.alive) return;
  r.alive = false;
  r.hitFlash = 0.6;
  s.flash = 0.45;
  s.lives--;
  if (s.lives < 0) { s.over = true; s.won = false; return; }
  r.respawn = 0.8;
}

function finalScore(s) {
  return Math.round(s.score);
}
