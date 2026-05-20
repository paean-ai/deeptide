// Pixel Puck - an air-hockey arcade. Drag your paddle freely in 2D, smash the
// puck through the top goal. First to POINTS_TO_WIN wins the match.

const VW = 360, VH = 480;
const FIELD_TOP = 60, FIELD_BOTTOM = 460;
const MID_Y = (FIELD_TOP + FIELD_BOTTOM) / 2;
const GOAL_W = 110;
const GOAL_X1 = (VW - GOAL_W) / 2;       // 125
const GOAL_X2 = GOAL_X1 + GOAL_W;        // 235

const PADDLE_R = 22;
const PUCK_R = 11;
const PUCK_MAX = 460;                    // puck speed cap, px/s
const FRICTION = 0.12;                   // per second (air-cushion)
const POINTS_TO_WIN = 5;
const SERVE_DELAY = 0.85;
const RESTITUTION = 1.05;                // slight speed-up on paddle hit

const LEVELS = [
  { name: ['Rookie', '新手'],   seed: 13,  cpuSpeed: 160, predict: 0.30, aim: 0.40 },
  { name: ['Cadet', '学员'],    seed: 41,  cpuSpeed: 195, predict: 0.45, aim: 0.55 },
  { name: ['Pro', '职业'],      seed: 92,  cpuSpeed: 230, predict: 0.60, aim: 0.68 },
  { name: ['Veteran', '老手'],  seed: 161, cpuSpeed: 265, predict: 0.72, aim: 0.78 },
  { name: ['Champion', '冠军'], seed: 248, cpuSpeed: 305, predict: 0.83, aim: 0.86 },
  { name: ['Master', '宗师'],   seed: 353, cpuSpeed: 345, predict: 0.92, aim: 0.94 },
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
  const s = {
    levelIndex, cfg,
    rng: seededRandom(cfg.seed),
    puck: { x: VW / 2, y: MID_Y, vx: 0, vy: 0 },
    player: { x: VW / 2, y: FIELD_BOTTOM - 60, px: VW / 2, py: FIELD_BOTTOM - 60 },
    cpu:    { x: VW / 2, y: FIELD_TOP + 60,    px: VW / 2, py: FIELD_TOP + 60 },
    playerScore: 0, cpuScore: 0,
    serveT: SERVE_DELAY, serveTo: null,
    over: false, won: false,
  };
  serve(s, s.rng() < 0.5 ? 'player' : 'cpu');
  return s;
}

function serve(s, to) {
  s.serveT = SERVE_DELAY;
  s.serveTo = to;
  s.puck.x = VW / 2;
  s.puck.y = MID_Y;
  s.puck.vx = 0; s.puck.vy = 0;
}
function launchServe(s) {
  const ang = (s.rng() - 0.5) * 0.6;        // mostly straight
  const dir = s.serveTo === 'cpu' ? -1 : 1; // serve toward receiver
  const v = 220;
  s.puck.vx = Math.sin(ang) * v;
  s.puck.vy = Math.cos(ang) * v * dir;
  s.serveTo = null;
}

// ---- player input -------------------------------------------------------
function setPlayerTarget(s, x, y) {
  // clamp to player half
  s.player.x = Math.max(PADDLE_R, Math.min(VW - PADDLE_R, x));
  s.player.y = Math.max(MID_Y + PADDLE_R + 2, Math.min(FIELD_BOTTOM - PADDLE_R, y));
}

// ---- CPU AI -------------------------------------------------------------
function stepCPU(s, dt) {
  let tx, ty;
  if (s.puck.vy < 0) {
    // puck heading toward CPU - predict x when it reaches a defense line
    const dy = (FIELD_TOP + PADDLE_R * 1.8) - s.puck.y;
    const t = dy / s.puck.vy;
    let px = s.puck.x + s.puck.vx * t;
    // bounce x against side walls (single fold)
    const W = VW - 2 * PUCK_R;
    let m = ((px - PUCK_R) % (2 * W) + 2 * W) % (2 * W);
    if (m >= W) m = 2 * W - m;
    px = PUCK_R + m;
    tx = s.cpu.x * (1 - s.cfg.predict) + px * s.cfg.predict;
    ty = FIELD_TOP + PADDLE_R + 8;
  } else {
    // puck heading away - move forward to strike (when puck is on CPU side)
    if (s.puck.y < MID_Y - 10) {
      tx = s.puck.x * s.cfg.aim + (VW / 2) * (1 - s.cfg.aim);
      ty = Math.min(s.puck.y + 6, MID_Y - PADDLE_R - 2);
    } else {
      tx = VW / 2; ty = FIELD_TOP + 60;
    }
  }
  const dx = tx - s.cpu.x, dy = ty - s.cpu.y;
  const dist = Math.hypot(dx, dy);
  const step = s.cfg.cpuSpeed * dt;
  if (dist <= step) { s.cpu.x = tx; s.cpu.y = ty; }
  else { s.cpu.x += dx / dist * step; s.cpu.y += dy / dist * step; }
  // clamp to CPU half
  s.cpu.x = Math.max(PADDLE_R, Math.min(VW - PADDLE_R, s.cpu.x));
  s.cpu.y = Math.max(FIELD_TOP + PADDLE_R, Math.min(MID_Y - PADDLE_R - 2, s.cpu.y));
}

// ---- collisions ---------------------------------------------------------
function collidePaddle(s, p, dt) {
  const dx = s.puck.x - p.x, dy = s.puck.y - p.y;
  const dist = Math.hypot(dx, dy);
  const min = PUCK_R + PADDLE_R;
  if (dist >= min || dist === 0) return;
  const nx = dx / dist, ny = dy / dist;
  // always push the puck out so circles never stay overlapped
  const overlap = min - dist;
  s.puck.x += nx * overlap;
  s.puck.y += ny * overlap;
  // reflect only when the puck is approaching the paddle in their RELATIVE
  // frame (paddle motion counts) - otherwise re-flips on every contact frame
  const pvx = (p.x - p.px) / dt;
  const pvy = (p.y - p.py) / dt;
  const rvDotN = (s.puck.vx - pvx) * nx + (s.puck.vy - pvy) * ny;
  if (rvDotN >= 0) return;
  // reflect puck velocity in the paddle's frame, then add paddle velocity
  s.puck.vx -= 2 * rvDotN * nx;
  s.puck.vy -= 2 * rvDotN * ny;
  s.puck.vx = s.puck.vx * RESTITUTION;
  s.puck.vy = s.puck.vy * RESTITUTION;
  capSpeed(s);
}

function capSpeed(s) {
  const sp = Math.hypot(s.puck.vx, s.puck.vy);
  if (sp > PUCK_MAX) { s.puck.vx *= PUCK_MAX / sp; s.puck.vy *= PUCK_MAX / sp; }
}

// ---- tick ---------------------------------------------------------------
function tick(s, dt, playerXY) {
  if (s.over) return;
  // remember previous paddle position for velocity transfer
  s.player.px = s.player.x; s.player.py = s.player.y;
  s.cpu.px = s.cpu.x;       s.cpu.py = s.cpu.y;
  if (playerXY) setPlayerTarget(s, playerXY.x, playerXY.y);
  stepCPU(s, dt);
  if (s.serveT > 0) {
    s.serveT -= dt;
    if (s.serveT <= 0) launchServe(s);
    return;
  }
  // friction
  s.puck.vx *= (1 - FRICTION * dt);
  s.puck.vy *= (1 - FRICTION * dt);
  // integrate
  s.puck.x += s.puck.vx * dt;
  s.puck.y += s.puck.vy * dt;
  // side walls
  if (s.puck.x < PUCK_R) { s.puck.x = PUCK_R; s.puck.vx = -s.puck.vx; }
  else if (s.puck.x > VW - PUCK_R) { s.puck.x = VW - PUCK_R; s.puck.vx = -s.puck.vx; }
  // top wall / top goal - the puck enters the goal as soon as its centre
  // crosses the goal line within the opening
  if (s.puck.y < FIELD_TOP + PUCK_R) {
    if (s.puck.x >= GOAL_X1 && s.puck.x <= GOAL_X2) {
      score(s, 'player');
      return;
    }
    s.puck.y = FIELD_TOP + PUCK_R;
    s.puck.vy = Math.abs(s.puck.vy);
  }
  // bottom wall / bottom goal
  if (s.puck.y > FIELD_BOTTOM - PUCK_R) {
    if (s.puck.x >= GOAL_X1 && s.puck.x <= GOAL_X2) {
      score(s, 'cpu');
      return;
    }
    s.puck.y = FIELD_BOTTOM - PUCK_R;
    s.puck.vy = -Math.abs(s.puck.vy);
  }
  collidePaddle(s, s.player, dt);
  collidePaddle(s, s.cpu, dt);
}

function score(s, who) {
  if (who === 'player') s.playerScore++;
  else s.cpuScore++;
  if (s.playerScore >= POINTS_TO_WIN) { s.over = true; s.won = true; return; }
  if (s.cpuScore >= POINTS_TO_WIN) { s.over = true; s.won = false; return; }
  serve(s, who === 'player' ? 'cpu' : 'player');
}
