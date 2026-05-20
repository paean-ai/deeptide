// Pixel Plinko - tap-to-drop ball through a triangular peg grid; the slot
// it lands in scores points. Ten balls per round; the round's score is
// compared against a per-board target to clear it.

const VW = 360, VH = 480;

const BOARD_X0   = 30;
const BOARD_X1   = 330;
const BOARD_Y0   = 60;
const PEG_Y0     = 90;
const BOARD_W    = BOARD_X1 - BOARD_X0;
const SLOT_Y     = 420;
const BALL_R     = 6;
const PEG_R      = 4;
const GRAVITY    = 720;          // px / s^2
const FRICTION_X = 0.985;        // horizontal damping per 60Hz tick
const BOUNCE     = 0.55;         // energy retained on a peg hit
const PIN_KICK   = 60;           // sideways nudge on contact
const MAX_VY     = 600;
const BALLS_PER_ROUND = 10;

// Each level: peg rows, slot values across the bottom, score target to clear.
const LEVELS = [
  { name: ['Carnival',  '集市'], rows: 9,  slotValues: [10, 50, 100, 50, 100, 50, 100, 50, 10],                 target: 250 },
  { name: ['Boardwalk', '木道'], rows: 10, slotValues: [5, 25, 50, 100, 250, 100, 50, 25, 5],                   target: 360 },
  { name: ['Arcade',    '游艺'], rows: 11, slotValues: [10, 25, 50, 100, 500, 100, 50, 25, 10],                 target: 480 },
  { name: ['Casino',    '赌场'], rows: 12, slotValues: [0, 50, 100, 250, 1000, 250, 100, 50, 0],                target: 720 },
  { name: ['Royale',    '皇家'], rows: 13, slotValues: [5, 50, 200, 500, 1500, 500, 200, 50, 5],                target: 1100 },
  { name: ['Olympus',   '神山'], rows: 14, slotValues: [0, 100, 250, 500, 2000, 500, 250, 100, 0],              target: 1600 },
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
  const rng = seededRandom(31 * (levelIndex + 1));
  // Pegs in a triangular pattern. Rows go from top (PEG_Y0) downward; each row
  // has rows+row+1 pegs... actually keep it simple: rows of (rows+1) and (rows)
  // alternating; we'll just use uniform spacing.
  const pegs = [];
  const stepY = (SLOT_Y - PEG_Y0 - 40) / (cfg.rows - 1);
  for (let r = 0; r < cfg.rows; r++) {
    const odd = r & 1;
    const count = 8 + (odd ? 1 : 0);            // alternating 8 / 9 pegs per row
    const stepX = BOARD_W / count;
    for (let i = 0; i < count; i++) {
      const x = BOARD_X0 + (i + 0.5) * stepX;
      const y = PEG_Y0 + r * stepY;
      pegs.push({ x, y });
    }
  }
  // Slot boundaries: even slot count + 1 walls.
  const nSlots = cfg.slotValues.length;
  const slotW = BOARD_W / nSlots;
  const slotWalls = [];
  for (let i = 0; i <= nSlots; i++) slotWalls.push(BOARD_X0 + i * slotW);
  return {
    levelIndex, cfg, rng,
    pegs, slotWalls,
    ball: null,                                  // {x, y, vx, vy} when in flight
    ballsLeft: BALLS_PER_ROUND,
    landed: [],                                  // {slot, value} per ball
    score: 0,
    over: false, won: false,
  };
}

// Drop a ball at world x (clamped to board).
function dropBall(s, x) {
  if (s.over) return false;
  if (s.ball) return false;
  if (s.ballsLeft <= 0) return false;
  const cx = Math.max(BOARD_X0 + BALL_R + 2, Math.min(BOARD_X1 - BALL_R - 2, x));
  s.ball = { x: cx, y: BOARD_Y0, vx: 0, vy: 0 };
  s.ballsLeft--;
  return true;
}

function tick(s, dt) {
  if (s.over) return;
  if (!s.ball) {
    // Round done?
    if (s.ballsLeft === 0) finishRound(s);
    return;
  }
  // 240Hz substep for stable peg collisions.
  const sub = 1 / 240;
  let remaining = dt;
  while (remaining > 0 && s.ball) {
    const step = Math.min(sub, remaining);
    substep(s, step);
    remaining -= step;
  }
}

function substep(s, dt) {
  const b = s.ball;
  // Gravity + horizontal damping.
  b.vy += GRAVITY * dt;
  if (b.vy > MAX_VY) b.vy = MAX_VY;
  b.vx *= Math.pow(FRICTION_X, dt * 60);
  b.x += b.vx * dt;
  b.y += b.vy * dt;
  // Side walls.
  if (b.x < BOARD_X0 + BALL_R) { b.x = BOARD_X0 + BALL_R; b.vx = Math.abs(b.vx) * 0.5; }
  if (b.x > BOARD_X1 - BALL_R) { b.x = BOARD_X1 - BALL_R; b.vx = -Math.abs(b.vx) * 0.5; }
  // Peg collisions.
  for (const p of s.pegs) {
    const dx = b.x - p.x, dy = b.y - p.y;
    const r = BALL_R + PEG_R;
    const d2 = dx * dx + dy * dy;
    if (d2 >= r * r) continue;
    const d = Math.sqrt(d2) || 0.0001;
    const nx = dx / d, ny = dy / d;
    const overlap = r - d;
    b.x += nx * overlap;
    b.y += ny * overlap;
    const v = b.vx * nx + b.vy * ny;
    if (v < 0) {
      b.vx -= 2 * v * nx * BOUNCE;
      b.vy -= 2 * v * ny * BOUNCE;
      // Tiny deterministic-ish sideways nudge based on impact point so the
      // ball isn't perfectly stable on a peg.
      b.vx += (b.x > p.x ? 1 : -1) * (PIN_KICK * (0.4 + s.rng() * 0.6));
    }
  }
  // Slot walls (vertical dividers at the bottom).
  for (const wx of s.slotWalls) {
    if (b.y + BALL_R > SLOT_Y && b.y - BALL_R < SLOT_Y + 50) {
      const dx = b.x - wx;
      if (Math.abs(dx) < BALL_R + 1) {
        if (dx > 0) { b.x = wx + BALL_R + 1; b.vx = Math.abs(b.vx) * 0.4; }
        else        { b.x = wx - BALL_R - 1; b.vx = -Math.abs(b.vx) * 0.4; }
      }
    }
  }
  // Reached the slot floor?
  if (b.y > SLOT_Y + 36) {
    landBall(s);
  }
}

function landBall(s) {
  const x = s.ball.x;
  // Which slot?
  const nSlots = s.cfg.slotValues.length;
  const slotW = BOARD_W / nSlots;
  let slot = ((x - BOARD_X0) / slotW) | 0;
  if (slot < 0) slot = 0;
  if (slot >= nSlots) slot = nSlots - 1;
  const value = s.cfg.slotValues[slot];
  s.landed.push({ x, slot, value });
  s.score += value;
  s.ball = null;
}

function finishRound(s) {
  s.over = true;
  s.won = s.score >= s.cfg.target;
}
