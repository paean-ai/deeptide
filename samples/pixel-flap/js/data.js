// Pixel Flap - one-button flappy-style arcade. Tap to flap, dodge the pipes,
// pass enough to clear the level.

const VW = 360, VH = 480;
const CEIL = 40, GROUND = VH - 56;
const BIRD_X = 100, BIRD_R = 11;
const PIPE_W = 44;
const GRAVITY = 1000;
const FLAP_V = -330;
const PIPE_SPACING = 130;
const SPAWN_AHEAD = VW + 20;        // x at which to spawn a new pipe

const LEVELS = [
  { name: ['Meadow', '草原'],    seed: 11,  count: 5,  gap: 132, speed: 100 },
  { name: ['Grove', '林地'],     seed: 41,  count: 8,  gap: 122, speed: 112 },
  { name: ['Canyon', '峡谷'],    seed: 88,  count: 12, gap: 114, speed: 124 },
  { name: ['Storm', '风暴'],     seed: 154, count: 16, gap: 104, speed: 138 },
  { name: ['Mountain', '高山'],  seed: 233, count: 20, gap: 96,  speed: 152 },
  { name: ['Aether', '天境'],    seed: 327, count: 24, gap: 88,  speed: 168 },
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
    bird: { y: (CEIL + GROUND) / 2, vy: 0, alive: true },
    pipes: [], spawnX: VW + 30,
    passed: 0, over: false, won: false, started: false,
  };
  // pre-seed a few pipes
  for (let k = 0; k < 3; k++) spawnPipe(s);
  return s;
}

function spawnPipe(s) {
  const margin = 30;
  const usable = (GROUND - CEIL) - s.cfg.gap - margin * 2;
  const gapY = CEIL + margin + s.rng() * usable;
  s.pipes.push({ x: s.spawnX, gapY, gapH: s.cfg.gap, passed: false });
  s.spawnX += PIPE_SPACING;
}

function flap(s) {
  if (s.over) return;
  s.started = true;
  if (s.bird.alive) s.bird.vy = FLAP_V;
}

function tick(s, dt) {
  if (s.over) return;
  if (!s.started) return;        // wait for first flap to start scroll
  // bird physics
  if (s.bird.alive) {
    s.bird.vy += GRAVITY * dt;
    s.bird.y += s.bird.vy * dt;
    if (s.bird.y < CEIL + BIRD_R || s.bird.y > GROUND - BIRD_R) {
      kill(s);
    }
  }
  // scroll pipes
  for (const p of s.pipes) p.x -= s.cfg.speed * dt;
  s.spawnX -= s.cfg.speed * dt;
  // spawn next pipe when needed
  while (s.passed + s.pipes.length < s.cfg.count + 3 && s.spawnX < SPAWN_AHEAD + PIPE_SPACING) {
    spawnPipe(s);
  }
  // collision + pass detection
  for (const p of s.pipes) {
    if (!s.bird.alive) break;
    if (circleRectHit(BIRD_X, s.bird.y, BIRD_R, p.x, CEIL, PIPE_W, p.gapY - CEIL) ||
        circleRectHit(BIRD_X, s.bird.y, BIRD_R, p.x, p.gapY + p.gapH, PIPE_W, GROUND - (p.gapY + p.gapH))) {
      kill(s);
      break;
    }
    if (!p.passed && p.x + PIPE_W < BIRD_X - BIRD_R) {
      p.passed = true;
      s.passed++;
      if (s.passed >= s.cfg.count) { s.over = true; s.won = true; }
    }
  }
  // drop off-screen pipes
  s.pipes = s.pipes.filter(p => p.x + PIPE_W > -10);
}

function circleRectHit(cx, cy, r, rx, ry, rw, rh) {
  const nx = Math.max(rx, Math.min(cx, rx + rw));
  const ny = Math.max(ry, Math.min(cy, ry + rh));
  const dx = cx - nx, dy = cy - ny;
  return dx * dx + dy * dy < r * r;
}

function kill(s) {
  s.bird.alive = false;
  s.over = true;
  s.won = false;
}
