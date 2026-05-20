// Pixel Archery - draw the bow, release, watch the arrow arc under gravity.
//
// Drag from the bow to set aim and power, release to loose. The arrow flies
// under a constant downward gravity plus the level's horizontal wind. Each
// hit on the concentric target rings (bull, inner, mid, outer) scores; you
// have 10 arrows per range and chase your best score.

const VW = 360, VH = 480;

const GRAVITY     = 480;          // px / s^2 on the arrow
const POWER_SCALE = 5.0;          // drag-pixels -> velocity (px/s)
const MAX_POWER   = 720;
const ARROW_LEN   = 20;
const ARROWS_PER_ROUND = 10;

// The bow lives near the bottom-left. Arrows spawn at the bow position.
const BOW_X = 50;
const BOW_Y = 360;

// Each level: target geometry (centre x, y) + target size (outer-ring radius)
// + wind (horizontal acceleration on the arrow in flight) + a description.
// Wind 'shift' means the wind value is re-rolled every arrow within [-cap, cap].
const LEVELS = [
  { name: ['Calm Field', '靖野'],     tx: 300, ty: 200, r: 32, wind:  0,   pattern: 'steady' },
  { name: ['Light Gust', '微风'],     tx: 300, ty: 200, r: 28, wind: 20,   pattern: 'steady' },
  { name: ['Crosswind', '横风'],      tx: 310, ty: 180, r: 26, wind: 40,   pattern: 'steady' },
  { name: ['Shifting Winds', '变风'], tx: 305, ty: 200, r: 26, wind: 60,   pattern: 'shift' },
  { name: ['Far Range', '远靶'],      tx: 330, ty: 160, r: 24, wind: 25,   pattern: 'steady' },
  { name: ['Storm Range', '暴风'],    tx: 325, ty: 180, r: 22, wind: 80,   pattern: 'shift' },
];
const LEVEL_COUNT = LEVELS.length;

// Concentric scoring rings, from inside out. radius is a fraction of the
// target's outer radius.
const RINGS = [
  { name: 'bull',  rad: 0.20, score: 10 },
  { name: 'inner', rad: 0.45, score: 8  },
  { name: 'mid',   rad: 0.75, score: 5  },
  { name: 'outer', rad: 1.00, score: 3  },
];

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
    rng: seededRandom(1009 * (levelIndex + 1) + 7),
    arrow: null,                  // {x,y,vx,vy} while flying
    aim: null,                    // {x,y} drag-end while aiming
    quiver: ARROWS_PER_ROUND,     // arrows left to shoot
    shot: 0,                      // arrows fired (incl. flying)
    score: 0,
    hits: [],                     // per-arrow {ringName?, points, x, y}
    activeWind: cfg.wind,
    over: false,
  };
}

function startAim(s, x, y) {
  if (s.over || s.arrow) return;
  if (s.quiver <= 0) return;
  s.aim = { x, y };
}
function updateAim(s, x, y) { if (s.aim) { s.aim.x = x; s.aim.y = y; } }

function releaseAim(s) {
  if (!s.aim || s.arrow) return;
  // Slingshot: drag AWAY from where you want the arrow to fly; release shoots
  // in the OPPOSITE direction. Demands an "away" drag to fire.
  const dx = s.aim.x - BOW_X;
  const dy = s.aim.y - BOW_Y;
  const len = Math.hypot(dx, dy);
  if (len < 10) { s.aim = null; return; }
  let vx = -dx * POWER_SCALE;
  let vy = -dy * POWER_SCALE;
  const speed = Math.hypot(vx, vy);
  if (speed > MAX_POWER) {
    const k = MAX_POWER / speed;
    vx *= k; vy *= k;
  }
  // Only allow shots that fly to the right (positive vx) - arrows mustn't
  // fly back over the archer's shoulder.
  if (vx <= 0) { s.aim = null; return; }
  s.arrow = { x: BOW_X, y: BOW_Y, vx, vy };
  s.aim = null;
  s.quiver--;
  s.shot++;
  // Per-arrow wind roll for 'shift' levels.
  if (s.cfg.pattern === 'shift') {
    s.activeWind = (s.rng() * 2 - 1) * s.cfg.wind;
  }
}

function tick(s, dt) {
  if (s.over || !s.arrow) return;
  // 240Hz substep to keep the trajectory smooth even at low frame rates.
  const sub = 1 / 240;
  let remaining = dt;
  while (remaining > 0) {
    const step = Math.min(sub, remaining);
    substep(s, step);
    remaining -= step;
    if (!s.arrow) return;
  }
}

function substep(s, dt) {
  const a = s.arrow;
  a.vy += GRAVITY * dt;
  a.vx += s.activeWind * dt;
  a.x += a.vx * dt;
  a.y += a.vy * dt;
  // Off the world to the right or bottom.
  if (a.y > VH + 40 || a.x > VW + 40 || a.x < -40) {
    recordHit(s, null);
    return;
  }
  // Hit-test against the target circle.
  const dx = a.x - s.cfg.tx;
  const dy = a.y - s.cfg.ty;
  const d  = Math.hypot(dx, dy);
  if (d <= s.cfg.r) {
    let ring = null;
    for (const r of RINGS) {
      if (d <= s.cfg.r * r.rad) { ring = r; break; }
    }
    recordHit(s, ring);
  }
}

function recordHit(s, ring) {
  const a = s.arrow;
  const hit = { x: a.x, y: a.y, points: ring ? ring.score : 0, ring: ring ? ring.name : null };
  s.hits.push(hit);
  s.score += hit.points;
  s.arrow = null;
  if (s.quiver === 0 && !s.arrow) {
    s.over = true;
  }
}
