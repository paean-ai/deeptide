// Pixel Rally - classic Pong vs CPU. Drag the bottom paddle, first to 5 wins.

const VW = 360, VH = 480;
const COURT_TOP = 60, COURT_BOTTOM = 460;
const COURT_H = COURT_BOTTOM - COURT_TOP;     // 400
const PADDLE_W = 64, PADDLE_H = 10;
const PLAYER_Y = COURT_BOTTOM - 24;
const CPU_Y = COURT_TOP + 24;
const BALL_R = 6;
const POINTS_TO_WIN = 5;
const SERVE_DELAY = 0.85;
const MAX_BOUNCE_ANGLE = 1.05;                // ~60 degrees

const LEVELS = [
  { name: ['Rookie', '新手'],   seed: 19,  cpuSpeed: 130, predict: 0.20, ballSpeed: 230 },
  { name: ['Cadet', '学员'],    seed: 47,  cpuSpeed: 165, predict: 0.35, ballSpeed: 255 },
  { name: ['Drill', '常规赛'],  seed: 92,  cpuSpeed: 200, predict: 0.50, ballSpeed: 280 },
  { name: ['Pro', '职业'],      seed: 161, cpuSpeed: 235, predict: 0.65, ballSpeed: 305 },
  { name: ['Champion', '冠军'], seed: 248, cpuSpeed: 270, predict: 0.80, ballSpeed: 330 },
  { name: ['Master', '宗师'],   seed: 356, cpuSpeed: 305, predict: 0.92, ballSpeed: 360 },
  { name: ['Ace', '王牌'],      seed: 471, cpuSpeed: 335, predict: 0.95, ballSpeed: 388 },
  { name: ['Phenom', '奇才'],   seed: 588, cpuSpeed: 365, predict: 0.97, ballSpeed: 416 },
  { name: ['Legend', '传奇'],   seed: 703, cpuSpeed: 395, predict: 0.99, ballSpeed: 444 },
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
    playerX: VW / 2, cpuX: VW / 2,
    ball: { x: VW / 2, y: COURT_TOP + COURT_H / 2, vx: 0, vy: 0 },
    playerScore: 0, cpuScore: 0,
    serveT: SERVE_DELAY, serveTo: null,
    over: false, won: false,
    lastHit: null,                              // 'player' | 'cpu' | null
    rallySpeed: cfg.ballSpeed,
  };
  serve(s, s.rng() < 0.5 ? 'player' : 'cpu');
  return s;
}

function serve(s, to) {
  s.serveT = SERVE_DELAY;
  s.serveTo = to;
  s.ball.x = VW / 2;
  s.ball.y = COURT_TOP + COURT_H / 2;
  s.ball.vx = 0; s.ball.vy = 0;
  s.rallySpeed = s.cfg.ballSpeed;
}
function launchServe(s) {
  // pick an angle within +/- 40 degrees from straight at the receiver
  const ang = (s.rng() - 0.5) * 1.4;          // about +/- 40 degrees
  const dy = s.serveTo === 'cpu' ? -1 : 1;    // serve toward receiver
  s.ball.vx = Math.sin(ang) * s.cfg.ballSpeed;
  s.ball.vy = Math.cos(ang) * s.cfg.ballSpeed * dy;
  s.serveTo = null;
}

// ---- CPU AI --------------------------------------------------------------
function predictX(s) {
  const b = s.ball;
  if (b.vy >= 0) return VW / 2;               // ball heading away -> rest near centre
  const t = (CPU_Y - b.y) / b.vy;             // > 0 since cpu above ball, vy < 0
  let x = b.x + b.vx * t;
  const W = VW;
  const period = 2 * W;
  let m = ((x % period) + period) % period;
  if (m >= W) m = 2 * W - m;
  return m;
}
function stepCPU(s, dt) {
  const target = lerp(VW / 2, predictX(s), s.cfg.predict)
    * s.cfg.predict + s.ball.x * (1 - s.cfg.predict);
  // actually: combine raw tracker with predictor
  const blended = s.ball.x * (1 - s.cfg.predict) + predictX(s) * s.cfg.predict;
  const max = s.cfg.cpuSpeed * dt;
  const d = blended - s.cpuX;
  if (Math.abs(d) <= max) s.cpuX = blended;
  else s.cpuX += Math.sign(d) * max;
  clampPaddle(s, 'cpuX');
}
function lerp(a, b, t) { return a + (b - a) * t; }

function clampPaddle(s, key) {
  const half = PADDLE_W / 2;
  s[key] = Math.max(half, Math.min(VW - half, s[key]));
}

// ---- tick ----------------------------------------------------------------
function tick(s, dt, playerX) {
  if (s.over) return;
  if (playerX !== undefined && playerX !== null) {
    s.playerX = playerX;
    clampPaddle(s, 'playerX');
  }
  stepCPU(s, dt);
  if (s.serveT > 0) {
    s.serveT -= dt;
    if (s.serveT <= 0) launchServe(s);
    return;
  }
  // move ball
  s.ball.x += s.ball.vx * dt;
  s.ball.y += s.ball.vy * dt;
  // side wall bounces
  if (s.ball.x < BALL_R) { s.ball.x = BALL_R; s.ball.vx = -s.ball.vx; }
  else if (s.ball.x > VW - BALL_R) { s.ball.x = VW - BALL_R; s.ball.vx = -s.ball.vx; }
  // paddle collisions
  paddleHit(s, 'player', PLAYER_Y, s.playerX, +1);
  paddleHit(s, 'cpu', CPU_Y, s.cpuX, -1);
  // scoring (ball exits top/bottom)
  if (s.ball.y > COURT_BOTTOM + 20) { s.cpuScore++; afterPoint(s, 'player'); }
  else if (s.ball.y < COURT_TOP - 20) { s.playerScore++; afterPoint(s, 'cpu'); }
}

function paddleHit(s, who, py, px, normal) {
  // normal: +1 = paddle below the ball (player); -1 = paddle above (cpu)
  // ball must be moving toward this paddle (vy*normal > 0)
  if (s.ball.vy * normal <= 0) return;
  // ball must be inside the paddle's vertical slab right now
  const top = py - PADDLE_H / 2 - BALL_R;
  const bot = py + PADDLE_H / 2 + BALL_R;
  if (s.ball.y < top || s.ball.y > bot) return;
  // horizontal overlap
  const dx = s.ball.x - px;
  if (Math.abs(dx) > PADDLE_W / 2 + BALL_R) return;
  if (s.lastHit === who) return;                // prevent re-hit jitter
  // bounce with angle based on hit offset
  const off = Math.max(-1, Math.min(1, dx / (PADDLE_W / 2)));
  const ang = off * MAX_BOUNCE_ANGLE;
  s.rallySpeed = Math.min(s.cfg.ballSpeed * 1.55, s.rallySpeed * 1.04);
  s.ball.vx = Math.sin(ang) * s.rallySpeed;
  s.ball.vy = Math.cos(ang) * s.rallySpeed * -normal;
  // snap to the surface side the ball came from to avoid double-hits
  s.ball.y = normal > 0 ? top : bot;
  s.lastHit = who;
}

function afterPoint(s, scorer) {
  s.lastHit = null;
  if (s.playerScore >= POINTS_TO_WIN) { s.over = true; s.won = true; return; }
  if (s.cpuScore >= POINTS_TO_WIN) { s.over = true; s.won = false; return; }
  serve(s, scorer === 'player' ? 'cpu' : 'player');
}
