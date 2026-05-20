// Pixel Helix - a top-down rotating disc tower. The ball stays pinned at
// 12 o'clock above the current disc and bounces forever; the player
// rotates the whole disc by dragging horizontally. When a gap segment
// rotates under the ball it falls through to the next disc below. A
// danger segment under the ball at the bounce-down peak is fatal.
//
// Each disc is 12 angular segments (30 deg each), one of:
//   0 = gap     (ball falls through)
//   1 = solid   (ball bounces back up)
//   2 = danger  (ball dies)

const VW = 360, VH = 480;

const SEGMENTS = 12;
const SEG_ANGLE = (Math.PI * 2) / SEGMENTS;

// Levels: tower depth + per-disc spec ranges + bounce cadence.
const LEVELS = [
  { name: ['Spring',   '春日'], depth: 10, gapMin: 4, gapMax: 6, dangerMax: 0, bounceHz: 1.6, seed: 11 },
  { name: ['Summer',   '盛夏'], depth: 14, gapMin: 3, gapMax: 5, dangerMax: 1, bounceHz: 1.9, seed: 22 },
  { name: ['Autumn',   '凉秋'], depth: 18, gapMin: 3, gapMax: 4, dangerMax: 1, bounceHz: 2.2, seed: 33 },
  { name: ['Winter',   '冬寒'], depth: 22, gapMin: 2, gapMax: 4, dangerMax: 2, bounceHz: 2.5, seed: 44 },
  { name: ['Eclipse',  '蚀'],   depth: 26, gapMin: 2, gapMax: 3, dangerMax: 2, bounceHz: 2.9, seed: 55 },
  { name: ['Singularity','奇点'], depth: 30, gapMin: 2, gapMax: 3, dangerMax: 3, bounceHz: 3.3, seed: 66 },
];
const LEVEL_COUNT = LEVELS.length;

// ---- helpers -----------------------------------------------------------
function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () { s = (s * 16807) % 2147483647; return (s - 1) / 2147483646; };
}

// Build one disc as an array of 12 segment values.
function buildDisc(rng, cfg) {
  const seg = new Array(SEGMENTS).fill(1);
  // Place gaps first (always at least cfg.gapMin so the ball can drop).
  const gaps = cfg.gapMin + Math.floor(rng() * (cfg.gapMax - cfg.gapMin + 1));
  for (let placed = 0; placed < gaps; ) {
    const i = Math.floor(rng() * SEGMENTS);
    if (seg[i] !== 1) continue;
    seg[i] = 0; placed++;
  }
  // Then danger segments, never adjacent to a gap (so the ball doesn't
  // brush into them mid-fall) and never replacing every solid (need
  // at least 2 solids left so the ball can still bounce).
  const dangers = cfg.dangerMax === 0 ? 0 : Math.floor(rng() * (cfg.dangerMax + 1));
  let solidsLeft = SEGMENTS - gaps;
  for (let placed = 0; placed < dangers && solidsLeft > 2; ) {
    const i = Math.floor(rng() * SEGMENTS);
    if (seg[i] !== 1) continue;
    const left = (i - 1 + SEGMENTS) % SEGMENTS;
    const right = (i + 1) % SEGMENTS;
    if (seg[left] === 0 || seg[right] === 0) continue;
    seg[i] = 2; placed++; solidsLeft--;
  }
  return seg;
}

// ---- runtime state -----------------------------------------------------
function buildGame(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const rng = seededRandom(cfg.seed);
  const discs = [];
  for (let i = 0; i < cfg.depth; i++) discs.push({
    segments: buildDisc(rng, cfg),
    rotation: rng() * Math.PI * 2,
  });
  return {
    levelIndex, cfg, discs,
    depth: cfg.depth,
    current: 0,                 // index of the disc the ball is currently on
    ballT: 0,                   // bounce phase 0..1 (1 = at lowest point above disc)
    bounceHz: cfg.bounceHz,
    falling: false,             // mid-transition to next disc
    fallT: 0,                   // 0..1 transition timer
    combo: 0,                   // discs cleared in a row without bouncing twice on same disc
    bouncedThisDisc: 0,         // bounces on the current disc since arrival
    score: 0,
    over: false, won: false,
    flash: 0,
  };
}

// ---- input -------------------------------------------------------------
function rotate(s, dAngle) {
  if (s.over || s.falling) return;
  const d = s.discs[s.current];
  d.rotation += dAngle;
}

// ---- tick --------------------------------------------------------------
function tick(s, dt) {
  if (s.over) return;
  s.flash = Math.max(0, s.flash - dt);
  if (s.falling) {
    s.fallT += dt * 3.5;
    if (s.fallT >= 1) {
      s.fallT = 0;
      s.falling = false;
      s.current++;
      s.bouncedThisDisc = 0;
      s.ballT = 0;
      if (s.current >= s.depth) {
        s.over = true; s.won = true;
        s.score += 500;
        s.flash = 0.55;
      }
    }
    return;
  }
  // Bounce phase advances at bounceHz cycles/sec.
  s.ballT += dt * s.bounceHz;
  while (s.ballT >= 1) {
    // Down-peak: check segment under ball.
    s.ballT -= 1;
    const segIdx = ballSegmentIndex(s);
    const t = s.discs[s.current].segments[segIdx];
    if (t === 2) { die(s); return; }
    if (t === 0) {
      // Fall through this gap to the next disc below.
      s.falling = true;
      s.fallT = 0;
      s.score += 25 + s.combo * 10;
      s.combo += 1;
      return;
    }
    // Solid: bounce. After one safe bounce on a disc the combo resets to 0
    // — combo only rewards "passing right through".
    s.bouncedThisDisc++;
    if (s.bouncedThisDisc > 1) s.combo = 0;
  }
}

// Index of the segment currently sitting at 12 o'clock (under the ball).
// Segments are laid out CW starting at angle -PI/2 (top), so the segment
// at the top is the one whose RELATIVE angle = (-PI/2 - rotation) maps to.
function ballSegmentIndex(s) {
  const d = s.discs[s.current];
  // Convert rotation to "how far the segments have rotated past the top".
  // Top angle = -PI/2 (canvas y-down). Segment k centre = (k + 0.5) * SEG_ANGLE.
  // After disc rotation r, segment k sits at angle r + (k + 0.5)*SEG_ANGLE - PI/2.
  // We want the k whose post-rotation centre is closest to -PI/2 (the top).
  let r = d.rotation % (Math.PI * 2);
  if (r < 0) r += Math.PI * 2;
  // Segment 0 originally at angle 0.5*SEG_ANGLE - PI/2 = -PI/2 + SEG_ANGLE/2.
  // Top = -PI/2 (or equivalently 3*PI/2). So we want k such that
  //   r + (k + 0.5)*SEG_ANGLE  =  0  (mod 2*PI)  // 12 o'clock in our system
  // => k = (-0.5 - r/SEG_ANGLE) mod SEGMENTS.
  let k = -0.5 - r / SEG_ANGLE;
  k = ((k % SEGMENTS) + SEGMENTS) % SEGMENTS;
  return Math.round(k) % SEGMENTS;
}

function die(s) {
  s.over = true; s.won = false;
  s.flash = 0.5;
}

function finalScore(s) {
  return s.score;
}
