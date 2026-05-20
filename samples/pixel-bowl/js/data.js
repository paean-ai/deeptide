// Pixel Bowl - 10-pin bowling. Top-down lane physics + standard scoring.
//
// The lane runs the height of the canvas. The player drags from the ball
// to set both aim and power, then releases to roll. Pins are knocked down
// by the rolling ball; once a pin falls, its area still propagates impact
// to any standing pin within FALL_RADIUS (a simple chain reaction).

const VW = 360, VH = 480;

const LANE_X      = 90;          // left edge of the lane (gutters outside)
const LANE_W      = 180;
const LANE_TOP    = 40;          // top of the play area (pin deck near here)
const LANE_BOT    = 460;         // ball spawn line
const BALL_R      = 8;
const PIN_R       = 7;
const FALL_RADIUS = 24;          // a fallen pin pushes nearby pins over
const FRICTION    = 0.985;       // per tick at 60Hz; ball decelerates
const MAX_POWER   = 540;         // px/s
const POWER_SCALE = 5.0;         // drag-pixels -> velocity scale
const MIN_SPEED   = 8;           // below this we consider the ball stopped

const FRAMES_PER_GAME = 10;

// Pin layout offsets (rows of 1, 2, 3, 4) at the head pin position. These
// are in world coords relative to the head pin centre.
const PIN_OFFSETS = (() => {
  const out = [];
  const dy = 18;
  const dx = 16;
  for (let row = 0; row < 4; row++) {
    for (let col = 0; col <= row; col++) {
      const x = (col - row / 2) * dx;
      const y = -row * dy;
      out.push([x, y]);
    }
  }
  return out;       // 1 + 2 + 3 + 4 = 10 pins
})();

function buildGame() {
  return {
    frame: 1,                      // 1..10
    throw: 1,                      // 1 or 2 (3rd allowed in frame 10 on strike/spare)
    frames: Array.from({ length: FRAMES_PER_GAME }, () => ({ throws: [], score: null })),
    pins: resetPins(),
    ball: null,                    // {x, y, vx, vy} while rolling
    aim: null,                     // {x, y} drag-end while aiming
    awaitingNext: false,
    over: false,
  };
}

function resetPins() {
  const headX = LANE_X + LANE_W / 2;
  const headY = LANE_TOP + 40;
  return PIN_OFFSETS.map(([dx, dy], i) => ({
    id: i, x: headX + dx, y: headY - dy, alive: true,
  }));
}

// ---- input: aim drag + release -----------------------------------------
function startAim(s, x, y) {
  if (s.ball || s.awaitingNext || s.over) return;
  s.aim = { x, y };
}
function updateAim(s, x, y) {
  if (!s.aim) return;
  s.aim.x = x; s.aim.y = y;
}
function releaseAim(s) {
  if (!s.aim || s.ball) return;
  // The ball spawns at the bottom centre of the lane and launches in the
  // direction of the swipe: drag UP from the ball -> ball rolls UP toward
  // the pins. Drag must have a meaningful upward (-y) component or it's
  // rejected.
  const sx = LANE_X + LANE_W / 2;
  const sy = LANE_BOT - 12;
  const dx = s.aim.x - sx;          // swipe direction
  const dy = s.aim.y - sy;
  const len = Math.hypot(dx, dy);
  if (len < 6) { s.aim = null; return; }
  if (dy > -6) { s.aim = null; return; }   // needs upward swipe
  let vx = dx * POWER_SCALE;
  let vy = dy * POWER_SCALE;
  const speed = Math.hypot(vx, vy);
  if (speed > MAX_POWER) {
    const k = MAX_POWER / speed;
    vx *= k; vy *= k;
  }
  s.ball = { x: sx, y: sy, vx, vy };
  s.aim = null;
}

// ---- tick: physics + scoring -------------------------------------------
function tick(s, dt) {
  if (s.over) return;
  if (!s.ball) return;
  // Physics step (60 Hz reference - dt may be variable).
  const steps = Math.max(1, (dt * 60) | 0 + 1);
  const sub = dt / steps;
  for (let k = 0; k < steps; k++) substep(s, sub);
}

function substep(s, dt) {
  const b = s.ball;
  // Friction (per second). FRICTION is per 60 Hz tick; convert.
  const factor = Math.pow(FRICTION, dt * 60);
  b.vx *= factor; b.vy *= factor;
  b.x += b.vx * dt;
  b.y += b.vy * dt;
  // Side walls (gutters) - bounce inward gently then keep going.
  if (b.x < LANE_X + BALL_R)              { b.x = LANE_X + BALL_R; b.vx = Math.abs(b.vx) * 0.5; }
  if (b.x > LANE_X + LANE_W - BALL_R)     { b.x = LANE_X + LANE_W - BALL_R; b.vx = -Math.abs(b.vx) * 0.5; }
  // Ball -> pin contact knocks the pin down.
  for (const p of s.pins) {
    if (!p.alive) continue;
    const dx = p.x - b.x, dy = p.y - b.y;
    const d2 = dx*dx + dy*dy;
    const r = BALL_R + PIN_R;
    if (d2 < r * r) {
      p.alive = false;
      // Chain: any standing pin within FALL_RADIUS of this knocked pin also falls.
      cascadeKnock(s, p.x, p.y);
      // Light deflection of the ball off the pin.
      const d = Math.sqrt(d2) || 1;
      b.vx -= (dx / d) * 12;
      b.vy -= (dy / d) * 12;
    }
  }
  // End conditions: ball off the top of the lane, or fell below MIN_SPEED.
  const speed = Math.hypot(b.vx, b.vy);
  if (b.y < LANE_TOP - 10 || (speed < MIN_SPEED && b.y < LANE_BOT - 60)) {
    finishThrow(s);
  }
}

// A fallen pin at (x,y) pushes over every standing pin within FALL_RADIUS,
// then those propagate further until no more pins fall.
function cascadeKnock(s, x, y) {
  let changed = true;
  let frontier = [{ x, y }];
  while (changed) {
    changed = false;
    const newFront = [];
    for (const p of s.pins) {
      if (!p.alive) continue;
      for (const f of frontier) {
        const dx = p.x - f.x, dy = p.y - f.y;
        if (dx*dx + dy*dy < FALL_RADIUS * FALL_RADIUS) {
          p.alive = false;
          newFront.push({ x: p.x, y: p.y });
          changed = true;
          break;
        }
      }
    }
    frontier = newFront;
  }
}

function knockedThisThrow(s) {
  const standing = s.pins.filter(p => p.alive).length;
  return 10 - standing - throwsKnockedSoFar(s);
}
function throwsKnockedSoFar(s) {
  const f = s.frames[s.frame - 1];
  let sum = 0;
  for (const v of f.throws) sum += v;
  return sum;
}

function finishThrow(s) {
  if (!s.ball) return;
  s.ball = null;
  const knocked = knockedThisThrow(s);
  const frame = s.frames[s.frame - 1];
  frame.throws.push(knocked);
  // Update scores from scratch each finish.
  recomputeScores(s);
  // Advance to the next throw / frame.
  const standing = s.pins.filter(p => p.alive).length;
  const last = (s.frame === FRAMES_PER_GAME);
  if (last) {
    // Frame 10 special: up to 3 throws if strike/spare.
    const t = frame.throws;
    let done = false;
    if (t.length === 3) done = true;
    else if (t.length === 2) {
      if (t[0] === 10 || t[0] + t[1] === 10) done = false;     // bonus ball
      else done = true;
    }
    if (done) { s.over = true; s.awaitingNext = false; }
    else {
      // Need another throw; if pins all down, reset.
      if (standing === 0) s.pins = resetPins();
      s.throw++;
      s.awaitingNext = true;
    }
  } else {
    if (frame.throws.length === 2 || standing === 0) {
      s.frame++;
      s.throw = 1;
      s.pins = resetPins();
      s.awaitingNext = true;
    } else {
      s.throw = 2;
      s.awaitingNext = true;
    }
  }
}

// Player presses "ready" or just throws again — we auto-clear awaitingNext
// on the next call to startAim. (See game.js.)
function clearWait(s) { s.awaitingNext = false; }

// Traditional 10-pin scoring with running totals.
function recomputeScores(s) {
  const frames = s.frames;
  // Flatten throws for lookahead (game.js handles frame 10 specially).
  const flat = [];
  for (let i = 0; i < FRAMES_PER_GAME; i++) {
    for (const v of frames[i].throws) flat.push(v);
  }
  let cursor = 0;
  let running = 0;
  for (let i = 0; i < FRAMES_PER_GAME; i++) {
    const f = frames[i];
    if (!f.throws.length) { f.score = null; continue; }
    if (i < FRAMES_PER_GAME - 1) {
      if (f.throws[0] === 10) {
        // Strike — need next two from flat.
        if (cursor + 2 < flat.length) {
          running += 10 + flat[cursor + 1] + flat[cursor + 2];
          f.score = running;
        } else f.score = null;
        cursor += 1;
      } else if (f.throws.length === 2) {
        const sum = f.throws[0] + f.throws[1];
        if (sum === 10) {
          if (cursor + 2 < flat.length) {
            running += 10 + flat[cursor + 2];
            f.score = running;
          } else f.score = null;
        } else {
          running += sum;
          f.score = running;
        }
        cursor += 2;
      } else {
        f.score = null;
        cursor += 1;
      }
    } else {
      // Frame 10: just sum its throws once finalized.
      let sum = 0;
      for (const v of f.throws) sum += v;
      const t = f.throws;
      const done =
        (t.length === 3) ||
        (t.length === 2 && t[0] !== 10 && t[0] + t[1] !== 10);
      f.score = done ? (running + sum) : null;
    }
  }
}

function gameScore(s) {
  for (let i = FRAMES_PER_GAME - 1; i >= 0; i--) {
    if (s.frames[i].score != null) return s.frames[i].score;
  }
  return 0;
}
