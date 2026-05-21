// Pixel Harpoon - a Pang / Buster Bros-style bouncing-orb shooter.
//
// Orbs bounce around the cavern under gravity. Fire a harpoon straight up:
// it hits the first orb in its column and splits it into two smaller orbs
// (the smallest just pop). An orb touching you costs a life. Clear every
// orb to finish the stage.

const VW = 360, VH = 480;

const CEIL_Y  = 36;
const FLOOR_Y = 422;
const WALL_L  = 8, WALL_R = VW - 8;

const GRAVITY = 540;
// Orb sizes 0 (tiny) .. 3 (huge): radius, floor-bounce speed, score.
const ORB_R      = [9, 15, 23, 32];
const ORB_BOUNCE = [232, 292, 352, 408];
const ORB_SCORE  = [50, 30, 20, 10];
const SPLIT_VX = 96;          // children fly apart this fast
const SPLIT_VY = 250;         // ...and pop upward this fast
const INIT_VX  = 64;          // an orb's starting drift

const HARPOON_SPEED = 640;
const PLAYER_W = 26, PLAYER_H = 30, PLAYER_SPEED = 172;
const START_LIVES = 3;
const HIT_INVULN = 2.4;

// Each level is a list of starting orbs [size, x]. y/velocity are derived.
const LEVELS = [
  { name: ['Drop In',   '落球'], orbs: [[2, 180]] },
  { name: ['Twin Fall', '双生'], orbs: [[2, 90], [2, 270]] },
  { name: ['The Giant', '巨球'], orbs: [[3, 180]] },
  { name: ['Crossfire', '交火'], orbs: [[3, 80], [3, 280], [1, 180]] },
  { name: ['Swarm',     '群涌'], orbs: [[2, 70], [2, 180], [2, 290], [1, 130], [1, 230]] },
  { name: ['Onslaught', '围攻'], orbs: [[3, 70], [3, 290], [2, 150], [2, 230]] },
];
const LEVEL_COUNT = LEVELS.length;

function buildGame(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const balls = cfg.orbs.map(([size, x], i) => ({
    size, x, y: CEIL_Y + ORB_R[size] + 18 + (i % 2) * 26,
    vx: (i % 2 === 0 ? 1 : -1) * INIT_VX,
    vy: 0, r: ORB_R[size],
  }));
  return {
    levelIndex, cfg,
    balls,
    player: { x: VW / 2, w: PLAYER_W, h: PLAYER_H },
    moveDir: 0,
    harpoon: null,
    lives: START_LIVES,
    score: 0,
    invuln: 1.2,
    flash: 0,
    over: false, won: false,
  };
}

// ---- input -------------------------------------------------------------
function setMove(s, dir) { if (!s.over) s.moveDir = dir; }

function fire(s) {
  if (s.over || s.harpoon) return false;
  s.harpoon = { x: s.player.x, tipY: FLOOR_Y };
  return true;
}

// ---- tick --------------------------------------------------------------
function tick(s, dt) {
  if (s.over) { s.flash = Math.max(0, s.flash - dt); return; }
  if (s.flash > 0) s.flash = Math.max(0, s.flash - dt);
  if (s.invuln > 0) s.invuln = Math.max(0, s.invuln - dt);

  // Player.
  s.player.x += s.moveDir * PLAYER_SPEED * dt;
  s.player.x = Math.max(WALL_L + PLAYER_W / 2, Math.min(WALL_R - PLAYER_W / 2, s.player.x));

  // Orbs: gravity, drift, bounce off walls / floor / ceiling.
  for (const b of s.balls) {
    b.vy += GRAVITY * dt;
    b.x += b.vx * dt;
    b.y += b.vy * dt;
    if (b.x - b.r < WALL_L) { b.x = WALL_L + b.r; b.vx = Math.abs(b.vx); }
    if (b.x + b.r > WALL_R) { b.x = WALL_R - b.r; b.vx = -Math.abs(b.vx); }
    if (b.y + b.r > FLOOR_Y) { b.y = FLOOR_Y - b.r; b.vy = -ORB_BOUNCE[b.size]; }
    if (b.y - b.r < CEIL_Y)  { b.y = CEIL_Y + b.r;  b.vy = Math.abs(b.vy); }
  }

  // Harpoon: a vertical wire from the floor up to tipY.
  if (s.harpoon) {
    const h = s.harpoon;
    h.tipY -= HARPOON_SPEED * dt;
    let hitIdx = -1;
    for (let i = 0; i < s.balls.length; i++) {
      const b = s.balls[i];
      if (Math.abs(b.x - h.x) <= b.r && b.y >= h.tipY - b.r) { hitIdx = i; break; }
    }
    if (hitIdx >= 0) { splitBall(s, hitIdx); s.harpoon = null; }
    else if (h.tipY <= CEIL_Y) s.harpoon = null;
  }

  // Orb vs player.
  if (s.invuln <= 0) {
    for (const b of s.balls) {
      if (circleHitsPlayer(b, s.player)) { loseLife(s); break; }
    }
  }

  if (s.balls.length === 0) { s.over = true; s.won = true; s.flash = 0.6; }
}

function splitBall(s, idx) {
  const b = s.balls[idx];
  s.score += ORB_SCORE[b.size];
  s.balls.splice(idx, 1);
  if (b.size > 0) {
    const ns = b.size - 1, nr = ORB_R[ns];
    s.balls.push({ size: ns, x: b.x, y: b.y, vx: -SPLIT_VX, vy: -SPLIT_VY, r: nr });
    s.balls.push({ size: ns, x: b.x, y: b.y, vx:  SPLIT_VX, vy: -SPLIT_VY, r: nr });
  }
}

function circleHitsPlayer(b, p) {
  const left = p.x - p.w / 2, right = p.x + p.w / 2;
  const top = FLOOR_Y - p.h;
  const cx = Math.max(left, Math.min(right, b.x));
  const cy = Math.max(top, Math.min(FLOOR_Y, b.y));
  const dx = b.x - cx, dy = b.y - cy;
  return dx * dx + dy * dy <= b.r * b.r;
}

function loseLife(s) {
  s.lives--;
  s.flash = 0.5;
  if (s.lives < 0) { s.over = true; s.won = false; return; }
  s.invuln = HIT_INVULN;
  s.harpoon = null;
}

function finalScore(s) {
  return s.score + (s.won ? Math.max(0, s.lives) * 120 : 0);
}
