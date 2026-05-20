// Pixel Pool - mini billiards. Cue ball + colored balls on a felt table with
// four corner pockets. Drag from the cue ball to aim + power; release to
// shoot. Friction slows everything until still. Pocket every colored ball
// in as few strokes as possible.

const VW = 360, VH = 480;

const TABLE_X0   = 24;
const TABLE_Y0   = 84;
const TABLE_X1   = 336;
const TABLE_Y1   = 396;
const TABLE_W    = TABLE_X1 - TABLE_X0;     // 312
const TABLE_H    = TABLE_Y1 - TABLE_Y0;     // 312
const BALL_R     = 8;
const POCKET_R   = 14;
const FRICTION   = 0.5;          // per second; vel *= FRICTION ^ dt
const MAX_POWER  = 720;          // px/s
const POWER_SCALE = 5.5;         // drag-pixels -> px/s
const MIN_SPEED  = 8;            // below this we consider a ball stopped

// Pocket positions (4 corners of the table).
const POCKETS = [
  { x: TABLE_X0, y: TABLE_Y0 },
  { x: TABLE_X1, y: TABLE_Y0 },
  { x: TABLE_X0, y: TABLE_Y1 },
  { x: TABLE_X1, y: TABLE_Y1 },
];

// Colored ball palette (used in the order of LEVELS[i].balls).
const BALL_COLORS = [
  '#f2cf3f', '#4a9be8', '#e8554f', '#9a6cd8',
  '#5fc06e', '#ef9b3e', '#4fd6d6', '#ff7db0',
];

const CUE_START = { x: 180, y: 350 };

// Each level: ball positions for the colored balls (in world coords) +
// suggested stroke budget. Levels get tighter as you go.
const LEVELS = [
  { name: ['Solo',     '独球'], strokes: 4, balls: [[180, 200]] },
  { name: ['Pair',     '双球'], strokes: 6, balls: [[150, 200], [210, 200]] },
  { name: ['Triangle', '三角'], strokes: 8, balls: [[180, 180], [165, 206], [195, 206]] },
  { name: ['Diamond',  '菱阵'], strokes: 9, balls: [[180, 160], [150, 200], [210, 200], [180, 240]] },
  { name: ['Star',     '星阵'], strokes: 10, balls: [[180, 160], [150, 200], [210, 200], [165, 240], [195, 240]] },
  { name: ['Rack',     '满架'], strokes: 12, balls: [
    [180, 160], [165, 186], [195, 186],
    [150, 212], [180, 212], [210, 212],
  ] },
];
const LEVEL_COUNT = LEVELS.length;

function buildGame(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const balls = [{
    kind: 'cue', x: CUE_START.x, y: CUE_START.y, vx: 0, vy: 0, alive: true, color: '#f8f5e8',
  }];
  for (let i = 0; i < cfg.balls.length; i++) {
    const [x, y] = cfg.balls[i];
    balls.push({
      kind: 'colored', x, y, vx: 0, vy: 0, alive: true,
      color: BALL_COLORS[i % BALL_COLORS.length],
    });
  }
  return {
    levelIndex, cfg, balls,
    state: 'aim',                   // 'aim' | 'rolling' | 'win' | 'lose'
    aim: null,                      // {x, y} drag-end while aiming
    strokes: 0,
    fouls: 0,
    pocketedThisStroke: [],
    started: false,
    won: false, over: false,
  };
}

// ---- input: drag-from-cue aim + release --------------------------------
function startAim(s, x, y) {
  if (s.state !== 'aim' || s.over) return;
  const cue = s.balls[0];
  if (!cue.alive) return;
  // Only start the aim when the touch is reasonably near the cue ball.
  s.aim = { x, y };
}
function updateAim(s, x, y) {
  if (!s.aim) return;
  s.aim.x = x; s.aim.y = y;
}
function releaseAim(s) {
  if (!s.aim || s.state !== 'aim') return;
  const cue = s.balls[0];
  const dx = s.aim.x - cue.x;
  const dy = s.aim.y - cue.y;
  const len = Math.hypot(dx, dy);
  if (len < 8) { s.aim = null; return; }            // tap = cancel aim
  // Slingshot: drag AWAY from where you want the ball to go; release shoots
  // in the OPPOSITE direction.
  let vx = -dx * POWER_SCALE;
  let vy = -dy * POWER_SCALE;
  const speed = Math.hypot(vx, vy);
  if (speed > MAX_POWER) {
    const k = MAX_POWER / speed;
    vx *= k; vy *= k;
  }
  cue.vx = vx; cue.vy = vy;
  s.aim = null;
  s.state = 'rolling';
  s.strokes++;
  s.pocketedThisStroke = [];
  s.started = true;
}

// ---- physics tick -------------------------------------------------------
function tick(s, dt) {
  if (s.over || s.state !== 'rolling') return;
  const sub = 1 / 240;
  let remaining = dt;
  while (remaining > 0) {
    const step = Math.min(sub, remaining);
    substep(s, step);
    remaining -= step;
  }
  // All balls stopped?
  if (s.balls.every(b => !b.alive || (b.vx * b.vx + b.vy * b.vy) < MIN_SPEED * MIN_SPEED)) {
    for (const b of s.balls) { b.vx = 0; b.vy = 0; }
    onStrokeEnd(s);
  }
}

function substep(s, dt) {
  for (const b of s.balls) {
    if (!b.alive) continue;
    // Friction.
    const fk = Math.pow(FRICTION, dt);
    b.vx *= fk; b.vy *= fk;
    b.x += b.vx * dt;
    b.y += b.vy * dt;
    // Table walls (rails).
    if (b.x < TABLE_X0 + BALL_R) { b.x = TABLE_X0 + BALL_R; b.vx = -b.vx * 0.85; }
    if (b.x > TABLE_X1 - BALL_R) { b.x = TABLE_X1 - BALL_R; b.vx = -b.vx * 0.85; }
    if (b.y < TABLE_Y0 + BALL_R) { b.y = TABLE_Y0 + BALL_R; b.vy = -b.vy * 0.85; }
    if (b.y > TABLE_Y1 - BALL_R) { b.y = TABLE_Y1 - BALL_R; b.vy = -b.vy * 0.85; }
  }
  // Ball-ball elastic collisions (equal mass, frictionless contact).
  for (let i = 0; i < s.balls.length; i++) {
    const a = s.balls[i];
    if (!a.alive) continue;
    for (let j = i + 1; j < s.balls.length; j++) {
      const b = s.balls[j];
      if (!b.alive) continue;
      collide(a, b);
    }
  }
  // Pocket check.
  for (const b of s.balls) {
    if (!b.alive) continue;
    for (const p of POCKETS) {
      const dx = p.x - b.x, dy = p.y - b.y;
      if (dx*dx + dy*dy < POCKET_R * POCKET_R) {
        b.alive = false;
        b.vx = 0; b.vy = 0;
        s.pocketedThisStroke.push(b);
        break;
      }
    }
  }
}

function collide(a, b) {
  const dx = b.x - a.x, dy = b.y - a.y;
  const d2 = dx * dx + dy * dy;
  const r = BALL_R + BALL_R;
  if (d2 >= r * r) return;
  const d = Math.sqrt(d2) || 0.0001;
  // Position correction so the balls stop overlapping.
  const overlap = r - d;
  const nx = dx / d, ny = dy / d;
  a.x -= nx * overlap * 0.5;
  a.y -= ny * overlap * 0.5;
  b.x += nx * overlap * 0.5;
  b.y += ny * overlap * 0.5;
  // Elastic exchange of the component along the contact normal.
  const va = a.vx * nx + a.vy * ny;
  const vb = b.vx * nx + b.vy * ny;
  const diff = va - vb;
  if (diff > 0) {                  // moving toward each other only
    a.vx -= diff * nx;
    a.vy -= diff * ny;
    b.vx += diff * nx;
    b.vy += diff * ny;
  }
}

function onStrokeEnd(s) {
  const cue = s.balls[0];
  if (!cue.alive) {
    // Cue scratched: respawn at the break point, foul +1.
    s.fouls++;
    cue.alive = true;
    cue.x = CUE_START.x; cue.y = CUE_START.y;
    cue.vx = 0; cue.vy = 0;
  }
  // Win check.
  const coloredLeft = s.balls.filter(b => b.kind === 'colored' && b.alive).length;
  if (coloredLeft === 0) {
    s.state = 'win';
    s.won = true; s.over = true;
    return;
  }
  // Lose: ran out of strokes.
  if (s.strokes >= s.cfg.strokes) {
    s.state = 'lose';
    s.over = true;
    return;
  }
  s.state = 'aim';
}

function gameScore(s) {
  const remaining = Math.max(0, s.cfg.strokes - s.strokes);
  const pocketed = s.cfg.balls.length - s.balls.filter(b => b.kind === 'colored' && b.alive).length;
  return pocketed * 100 + remaining * 50 - s.fouls * 30;
}
