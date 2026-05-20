// Pixel Sumo - top-down circular arena. Drag from your wrestler to dash;
// elastic collisions push both wrestlers. First out of the ring loses.

const VW = 360, VH = 480;

const RING_CX  = 180;
const RING_CY  = 260;
const RING_R   = 140;
const WRESTLER_R = 14;
const FRICTION = 0.18;            // per second; vel *= FRICTION ^ dt
const MAX_DASH = 480;             // px/s on a player dash
const DASH_SCALE = 5.5;           // drag-pixels -> velocity
const AI_BURST = 360;             // base AI charge speed
const MIN_SPEED = 6;
const PLAYER_MASS = 1;
const AI_MASS    = 1;

// Each level: ai aggression (how often the AI charges) + ai strength.
const LEVELS = [
  { name: ['Novice',   '新手'], aiCd: 1.8, aiPower: 280 },
  { name: ['Rookie',   '新锐'], aiCd: 1.4, aiPower: 320 },
  { name: ['Junior',   '少壮'], aiCd: 1.1, aiPower: 360 },
  { name: ['Senior',   '老成'], aiCd: 0.9, aiPower: 400 },
  { name: ['Champion', '冠军'], aiCd: 0.7, aiPower: 440 },
  { name: ['Yokozuna', '横纲'], aiCd: 0.55, aiPower: 480 },
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
    rng: seededRandom(7 * (levelIndex + 1) + 41),
    // Player starts left of centre, AI starts right.
    player: { x: RING_CX - 50, y: RING_CY, vx: 0, vy: 0, mass: PLAYER_MASS, alive: true },
    ai:     { x: RING_CX + 50, y: RING_CY, vx: 0, vy: 0, mass: AI_MASS, alive: true },
    aiTimer: cfg.aiCd * 0.6,
    aim: null,                     // {x, y} during a drag
    started: false,
    over: false, won: false,
    elapsed: 0,
  };
}

// ---- input -------------------------------------------------------------
function startAim(s, x, y) {
  if (s.over || !s.player.alive) return;
  s.aim = { x, y };
}
function updateAim(s, x, y) { if (s.aim) { s.aim.x = x; s.aim.y = y; } }

function releaseAim(s) {
  if (!s.aim || s.over || !s.player.alive) return;
  // Drag-AWAY slingshot: the player dashes OPPOSITE the drag direction.
  const dx = s.aim.x - s.player.x;
  const dy = s.aim.y - s.player.y;
  const len = Math.hypot(dx, dy);
  if (len < 8) { s.aim = null; return; }
  let vx = -dx * DASH_SCALE;
  let vy = -dy * DASH_SCALE;
  const sp = Math.hypot(vx, vy);
  if (sp > MAX_DASH) { vx *= MAX_DASH / sp; vy *= MAX_DASH / sp; }
  s.player.vx += vx;
  s.player.vy += vy;
  s.aim = null;
  s.started = true;
}

// ---- tick --------------------------------------------------------------
function tick(s, dt) {
  if (s.over) return;
  if (!s.started) return;
  s.elapsed += dt;
  // 240Hz substep for clean collision resolution.
  const sub = 1 / 240;
  let remaining = dt;
  while (remaining > 0 && !s.over) {
    const step = Math.min(sub, remaining);
    substep(s, step);
    remaining -= step;
  }
  // AI timer.
  s.aiTimer -= dt;
  if (s.aiTimer <= 0 && s.ai.alive) {
    aiCharge(s);
    s.aiTimer = s.cfg.aiCd * (0.8 + s.rng() * 0.4);
  }
}

function substep(s, dt) {
  const wrestlers = [s.player, s.ai];
  for (const w of wrestlers) {
    if (!w.alive) continue;
    const f = Math.pow(FRICTION, dt);
    w.vx *= f; w.vy *= f;
    w.x += w.vx * dt;
    w.y += w.vy * dt;
  }
  // Wrestler-wrestler elastic collision.
  collide(s.player, s.ai);
  // Falling out of the ring.
  for (const w of wrestlers) {
    if (!w.alive) continue;
    if (Math.hypot(w.x - RING_CX, w.y - RING_CY) > RING_R) {
      w.alive = false;
      // Brief free slide off-screen before ending the match.
    }
  }
  if (!s.over) {
    if (!s.player.alive) { s.over = true; s.won = false; }
    else if (!s.ai.alive) { s.over = true; s.won = true; }
  }
}

function collide(a, b) {
  const dx = b.x - a.x, dy = b.y - a.y;
  const d2 = dx * dx + dy * dy;
  const r = WRESTLER_R + WRESTLER_R;
  if (d2 >= r * r) return;
  const d = Math.sqrt(d2) || 0.0001;
  const nx = dx / d, ny = dy / d;
  const overlap = r - d;
  // Position correction split by mass.
  const totalM = a.mass + b.mass;
  a.x -= nx * overlap * (b.mass / totalM);
  a.y -= ny * overlap * (b.mass / totalM);
  b.x += nx * overlap * (a.mass / totalM);
  b.y += ny * overlap * (a.mass / totalM);
  // Elastic exchange along the contact normal.
  const va = a.vx * nx + a.vy * ny;
  const vb = b.vx * nx + b.vy * ny;
  const va2 = (va * (a.mass - b.mass) + 2 * b.mass * vb) / totalM;
  const vb2 = (vb * (b.mass - a.mass) + 2 * a.mass * va) / totalM;
  a.vx += (va2 - va) * nx; a.vy += (va2 - va) * ny;
  b.vx += (vb2 - vb) * nx; b.vy += (vb2 - vb) * ny;
}

// ---- AI ---------------------------------------------------------------
// The AI charges along the line from itself toward the player.
function aiCharge(s) {
  if (!s.ai.alive || !s.player.alive) return;
  const dx = s.player.x - s.ai.x;
  const dy = s.player.y - s.ai.y;
  const d = Math.hypot(dx, dy) || 1;
  const ux = dx / d, uy = dy / d;
  s.ai.vx += ux * s.cfg.aiPower;
  s.ai.vy += uy * s.cfg.aiPower;
  // Cap.
  const sp = Math.hypot(s.ai.vx, s.ai.vy);
  if (sp > MAX_DASH) { s.ai.vx *= MAX_DASH / sp; s.ai.vy *= MAX_DASH / sp; }
}
