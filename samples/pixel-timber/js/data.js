// Pixel Timber - a chop-and-dodge arcade.
//
// A tree trunk is a stack of logs. The lumberjack stands on the left or
// right of the trunk. Every tap chops the bottom log: the stack drops one
// notch, a fresh log appears on top, and the score ticks up. Some logs
// carry a branch on the left or the right - if the new bottom log's branch
// is on the side the lumberjack stands, it knocks them out. A stamina bar
// drains the whole time and is topped up by each chop, so you must keep a
// steady rhythm. Endless; chase the best score.

const VW = 360, VH = 480;

const SEG_H    = 52;          // pixel height of one trunk log
const TRUNK_W  = 64;
const STACK    = 11;          // logs kept in the internal stack
const VISIBLE  = 8;           // logs drawn on screen

const SIDE_LEFT = 0, SIDE_RIGHT = 1;
const BR_NONE = 0, BR_LEFT = 1, BR_RIGHT = 2;

// Stamina: starts part-full, drains over time (faster as the score climbs),
// and is refilled a chunk by every chop.
const STAMINA_START = 0.78;
const CHOP_REFILL   = 0.16;
const BASE_DRAIN    = 0.26;   // per second at score 0
const DRAIN_RAMP    = 0.0045; // extra drain per second per point of score

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () { s = (s * 16807) % 2147483647; return (s - 1) / 2147483646; };
}

// A log's branch. Pure random - every single log has at most one branch, so
// there is always a safe side; the challenge is reading it in time.
function genBranch(rng) {
  const r = rng();
  if (r < 0.44) return BR_NONE;
  return r < 0.72 ? BR_LEFT : BR_RIGHT;
}

function buildGame(seed) {
  const rng = seededRandom(seed || ((Date.now() & 0x7fffffff) || 1));
  const segments = [];
  // The bottom two logs are always clear so the first chop can't catch you.
  for (let i = 0; i < STACK; i++) {
    segments.push({ branch: i < 2 ? BR_NONE : genBranch(rng) });
  }
  return {
    rng,
    segments,
    side: SIDE_LEFT,
    score: 0,
    stamina: STAMINA_START,
    over: false,
    chopT: 0,                 // axe-swing animation timer
    shake: 0,                 // screen-shake timer
    flyLog: null,             // {x, vx, vy, rot, rotV} chopped log flying off
    hitSide: -1,              // which side the knock-out branch was on
  };
}

// Chop from the given side. The lumberjack steps to that side, fells the
// bottom log, the stack drops, and a new log is generated on top.
function chop(s, side) {
  if (s.over) return;
  s.side = side;
  const felled = s.segments.shift();
  s.segments.push({ branch: genBranch(s.rng) });
  s.score++;
  s.stamina = Math.min(1, s.stamina + CHOP_REFILL);
  s.chopT = 0.16;
  // The felled log tumbles away from the lumberjack.
  const dir = side === SIDE_LEFT ? 1 : -1;
  s.flyLog = { x: VW / 2, vx: dir * 230, vy: -90, rot: 0, rotV: dir * 9 };
  // The log that just became the bottom decides your fate.
  const bottom = s.segments[0].branch;
  if ((bottom === BR_LEFT && side === SIDE_LEFT) ||
      (bottom === BR_RIGHT && side === SIDE_RIGHT)) {
    s.over = true;
    s.hitSide = side;
    s.shake = 0.5;
  }
  return felled;
}

function tick(s, dt) {
  if (s.chopT > 0) s.chopT = Math.max(0, s.chopT - dt);
  if (s.shake > 0) s.shake = Math.max(0, s.shake - dt);
  if (s.flyLog) {
    const f = s.flyLog;
    f.vy += 900 * dt;
    f.x += f.vx * dt;
    f.y = (f.y || 410) + f.vy * dt;
    f.rot += f.rotV * dt;
    if (f.y > VH + 80) s.flyLog = null;
  }
  if (s.over) return;
  const drain = BASE_DRAIN + s.score * DRAIN_RAMP;
  s.stamina -= drain * dt;
  if (s.stamina <= 0) {
    s.stamina = 0;
    s.over = true;
    s.hitSide = -1;           // ran out of time rather than hit by a branch
  }
}

function finalScore(s) { return s.score; }
