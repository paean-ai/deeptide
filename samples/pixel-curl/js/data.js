// Pixel Curl - top-down curling. Slide stones down an ice sheet toward a
// target "house" at the top; an AI opponent throws stones too, alternating
// turns. After all 8 stones (4 per side) are thrown, score = how many of
// the closest stones inside the house belong to one player before the
// other's first stone interrupts.

const VW = 360, VH = 480;

const SHEET_X0   = 40;
const SHEET_X1   = 320;
const SHEET_Y0   = 30;
const SHEET_Y1   = 440;
const HOUSE_X    = (SHEET_X0 + SHEET_X1) / 2;
const HOUSE_Y    = 90;
const HOUSE_R    = 50;           // outer ring
const RINGS      = [50, 36, 22, 9];  // outer ring radius, then inner rings
const STONE_R    = 9;
const SPAWN_Y    = 420;          // y where the throwing arc starts
const FRICTION   = 0.32;         // per second; vel *= FRICTION ^ dt
const MAX_POWER  = 720;
const POWER_SCALE = 4.4;         // drag pixels -> px/s
const MIN_SPEED  = 6;            // below this we consider a stone stopped
const STONES_PER_SIDE = 4;

// Each level: AI accuracy as a stddev (px) on its target landing point.
// Lower = sharper aim.
const LEVELS = [
  { name: ['Rookie',     '新手'], aiSigma: 32 },
  { name: ['Skip',       '副将'], aiSigma: 26 },
  { name: ['Lead',       '主帅'], aiSigma: 20 },
  { name: ['Vice',       '副帅'], aiSigma: 16 },
  { name: ['Champion',   '冠军'], aiSigma: 12 },
  { name: ['Olympic',    '奥运'], aiSigma: 9  },
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

function buildGame(levelIndex) {
  const cfg = LEVELS[levelIndex];
  return {
    levelIndex, cfg,
    rng: seededRandom(401 * (levelIndex + 1) + 19),
    stones: [],                  // {x, y, vx, vy, alive, owner: 'p' | 'a'}
    flying: null,                // ref to the live stone
    aim: null,                   // {x, y} during a player drag
    turn: 'p',                   // 'p' (player) or 'a' (AI)
    playerLeft: STONES_PER_SIDE,
    aiLeft: STONES_PER_SIDE,
    aiTimer: 0,
    over: false,
    won: false,
    score: { p: 0, a: 0 },
  };
}

function startAim(s, x, y) {
  if (s.over || s.flying || s.turn !== 'p') return;
  if (s.playerLeft <= 0) return;
  s.aim = { x, y };
}
function updateAim(s, x, y) { if (s.aim) { s.aim.x = x; s.aim.y = y; } }

function releaseAim(s) {
  if (!s.aim || s.flying || s.turn !== 'p') return;
  const sx = (SHEET_X0 + SHEET_X1) / 2, sy = SPAWN_Y;
  // Drag-AWAY slingshot: pull back, release shoots forward (up).
  const dx = s.aim.x - sx;
  const dy = s.aim.y - sy;
  const len = Math.hypot(dx, dy);
  if (len < 10) { s.aim = null; return; }
  // Must be a "downward" drag (drag end is below spawn), i.e. dy > 0.
  if (dy < 10) { s.aim = null; return; }
  let vx = -dx * POWER_SCALE;
  let vy = -dy * POWER_SCALE;
  const speed = Math.hypot(vx, vy);
  if (speed > MAX_POWER) { vx *= MAX_POWER / speed; vy *= MAX_POWER / speed; }
  launchStone(s, sx, sy, vx, vy, 'p');
  s.playerLeft--;
  s.aim = null;
}

function launchStone(s, x, y, vx, vy, owner) {
  const st = { x, y, vx, vy, alive: true, owner };
  s.stones.push(st);
  s.flying = st;
}

function tick(s, dt) {
  if (s.over) return;
  // Physics substep for stable collisions.
  const sub = 1 / 240;
  let remaining = dt;
  while (remaining > 0) {
    const step = Math.min(sub, remaining);
    substep(s, step);
    remaining -= step;
  }
  // Stone resolution: when all stones are stopped, swap turns.
  if (s.flying && allStopped(s)) {
    s.flying = null;
    advanceTurn(s);
  }
  // AI turn pump: pick a target + power after a short delay.
  if (!s.flying && s.turn === 'a' && s.aiLeft > 0 && !s.over) {
    s.aiTimer -= dt;
    if (s.aiTimer <= 0) {
      aiThrow(s);
      s.aiLeft--;
    }
  }
}

function substep(s, dt) {
  // Friction.
  const f = Math.pow(FRICTION, dt);
  for (const st of s.stones) {
    if (!st.alive) continue;
    st.vx *= f; st.vy *= f;
    st.x  += st.vx * dt;
    st.y  += st.vy * dt;
    // Side walls reflect.
    if (st.x < SHEET_X0 + STONE_R) { st.x = SHEET_X0 + STONE_R; st.vx = -st.vx * 0.6; }
    if (st.x > SHEET_X1 - STONE_R) { st.x = SHEET_X1 - STONE_R; st.vx = -st.vx * 0.6; }
    // Off the back of the sheet (top) or bottom = removed.
    if (st.y < SHEET_Y0 - STONE_R || st.y > SHEET_Y1 + STONE_R) st.alive = false;
  }
  // Pairwise elastic collisions.
  for (let i = 0; i < s.stones.length; i++) {
    const a = s.stones[i];
    if (!a.alive) continue;
    for (let j = i + 1; j < s.stones.length; j++) {
      const b = s.stones[j];
      if (!b.alive) continue;
      collide(a, b);
    }
  }
}

function collide(a, b) {
  const dx = b.x - a.x, dy = b.y - a.y;
  const d2 = dx * dx + dy * dy;
  const r = STONE_R + STONE_R;
  if (d2 >= r * r) return;
  const d = Math.sqrt(d2) || 0.0001;
  const overlap = r - d;
  const nx = dx / d, ny = dy / d;
  a.x -= nx * overlap * 0.5;
  a.y -= ny * overlap * 0.5;
  b.x += nx * overlap * 0.5;
  b.y += ny * overlap * 0.5;
  const va = a.vx * nx + a.vy * ny;
  const vb = b.vx * nx + b.vy * ny;
  const diff = va - vb;
  if (diff > 0) {
    a.vx -= diff * nx; a.vy -= diff * ny;
    b.vx += diff * nx; b.vy += diff * ny;
  }
}

function allStopped(s) {
  for (const st of s.stones) {
    if (!st.alive) continue;
    if (st.vx * st.vx + st.vy * st.vy > MIN_SPEED * MIN_SPEED) return false;
  }
  return true;
}

function advanceTurn(s) {
  // Swap to the other player if they have stones left.
  if (s.turn === 'p' && s.aiLeft > 0)      { s.turn = 'a'; s.aiTimer = 0.7; }
  else if (s.turn === 'a' && s.playerLeft > 0) { s.turn = 'p'; }
  else if (s.turn === 'p' && s.aiLeft === 0)   { s.turn = 'a'; }
  else if (s.turn === 'a' && s.playerLeft === 0) { s.turn = 'p'; }
  // End of game?
  if (s.playerLeft === 0 && s.aiLeft === 0) finishGame(s);
}

function finishGame(s) {
  // Score: count the player's stones inside the house that are CLOSER
  // to centre than every opposing stone. (Standard curling end score.)
  const live = s.stones.filter(st => st.alive);
  const inHouse = live.filter(st => Math.hypot(st.x - HOUSE_X, st.y - HOUSE_Y) <= HOUSE_R);
  inHouse.sort((a, b) => dist(a) - dist(b));
  function dist(st) { return Math.hypot(st.x - HOUSE_X, st.y - HOUSE_Y); }
  if (!inHouse.length) { s.score = { p: 0, a: 0 }; s.won = false; }
  else {
    const winner = inHouse[0].owner;
    let count = 0;
    for (const st of inHouse) {
      if (st.owner === winner) count++;
      else break;
    }
    s.score = { p: winner === 'p' ? count : 0, a: winner === 'a' ? count : 0 };
    s.won = winner === 'p';
  }
  s.over = true;
}

// ---- AI throws ---------------------------------------------------------
// The AI aims at HOUSE_X, HOUSE_Y with Gaussian noise of stddev aiSigma.
// Power is tuned to roughly land at the target point.
function aiThrow(s) {
  const sx = (SHEET_X0 + SHEET_X1) / 2, sy = SPAWN_Y;
  const tx = HOUSE_X + gauss(s.rng) * s.cfg.aiSigma;
  const ty = HOUSE_Y + gauss(s.rng) * s.cfg.aiSigma;
  // Calibrated velocity to land at (tx, ty) under FRICTION (per-second).
  // Friction integrated: dist = v0 / k where k = -ln(FRICTION) per s.
  // So v0 ~= dist * k. Use a slightly larger value because of variable dt.
  const dx = tx - sx, dy = ty - sy;
  const dist = Math.hypot(dx, dy);
  const k = -Math.log(FRICTION);
  let speed = dist * k * 1.05;
  if (speed > MAX_POWER) speed = MAX_POWER;
  const ux = dx / dist, uy = dy / dist;
  launchStone(s, sx, sy, ux * speed, uy * speed, 'a');
}
function gauss(rng) {
  // Box-Muller; clamp to ±3 sigma for stability.
  let u = 0, v = 0;
  while (u === 0) u = rng();
  while (v === 0) v = rng();
  const z = Math.sqrt(-2 * Math.log(u)) * Math.cos(2 * Math.PI * v);
  return Math.max(-3, Math.min(3, z));
}
