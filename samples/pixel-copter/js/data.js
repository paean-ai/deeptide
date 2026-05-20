// Pixel Copter - hold-to-rise side-scroller through a procedural cave.
//
// The cave is a top + bottom polyline that scrolls right-to-left. Holding
// the pointer applies continuous upward thrust; releasing lets gravity
// take over. Hit the cave walls or a pillar and you crash. Score = farthest
// distance reached.

const VW = 360, VH = 480;

const COPTER_X    = 80;
const COPTER_R    = 10;
const GRAVITY     = 700;        // px / s^2 downward
const THRUST_AY   = -900;       // additional acceleration while holding
const MAX_VY      = 520;        // terminal fall speed
const SAMPLE_DX   = 12;         // cave samples every 12 px
const PILLAR_W    = 28;

// Each level: scroll speed (px/s) and gap settings. Gap shrinks over distance.
const LEVELS = [
  { name: ['Bay Pass',     '海湾'],   speed: 110, gap: 170, gapMin: 130, pillarEvery: 1400, seed: 13 },
  { name: ['Forge Caves',  '熔坑'],   speed: 130, gap: 160, gapMin: 120, pillarEvery: 1200, seed: 71 },
  { name: ['Glacier Bore', '冰川道'], speed: 150, gap: 150, gapMin: 110, pillarEvery: 1000, seed: 137 },
  { name: ['Sky Tunnel',   '云道'],   speed: 170, gap: 145, gapMin: 100, pillarEvery: 900,  seed: 191 },
  { name: ['Storm Vault',  '风穹'],   speed: 200, gap: 140, gapMin: 95,  pillarEvery: 800,  seed: 257 },
  { name: ['Abyss Run',    '深渊'],   speed: 230, gap: 135, gapMin: 88,  pillarEvery: 720,  seed: 331 },
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
    copter: { y: VH / 2, vy: 0, alive: true, rotor: 0 },
    distance: 0,                  // px travelled
    score: 0,
    over: false, won: false, started: false,
    thrust: false,
    samples: [],                  // {x, midY, gap} every SAMPLE_DX in world coords
    pillars: [],                  // {x, gapY, gapH}
    nextPillarAt: cfg.pillarEvery,
    midY: VH / 2,
    targetMidY: VH / 2,
    midDrift: 0,
  };
  // Seed enough samples to cover from -COPTER_X off-screen to right edge + a buffer.
  for (let x = -COPTER_X; x <= VW + SAMPLE_DX * 4; x += SAMPLE_DX) {
    s.samples.push({ x, midY: VH / 2, gap: cfg.gap });
  }
  return s;
}

function setInput(s, thrust) {
  if (s.over) return;
  if (thrust) s.started = true;
  s.thrust = !!thrust;
}

function tick(s, dt) {
  if (s.over) return;
  if (!s.started) return;
  s.copter.rotor += dt * 18;
  // Physics.
  let ay = GRAVITY;
  if (s.thrust) ay += THRUST_AY;
  s.copter.vy += ay * dt;
  if (s.copter.vy > MAX_VY) s.copter.vy = MAX_VY;
  if (s.copter.vy < -MAX_VY) s.copter.vy = -MAX_VY;
  s.copter.y += s.copter.vy * dt;
  // Scroll the world: shift all samples left, drop ones off-screen, spawn ahead.
  const scroll = s.cfg.speed * dt;
  s.distance += scroll;
  for (const samp of s.samples) samp.x -= scroll;
  for (const p of s.pillars) p.x -= scroll;
  while (s.samples.length && s.samples[0].x < -COPTER_X - SAMPLE_DX) s.samples.shift();
  const lastX = s.samples.length ? s.samples[s.samples.length - 1].x : 0;
  while (s.samples.length === 0 || s.samples[s.samples.length - 1].x < VW + SAMPLE_DX * 4) {
    addSample(s);
  }
  // Pillar spawning.
  s.nextPillarAt -= scroll;
  if (s.nextPillarAt <= 0) {
    spawnPillar(s);
    s.nextPillarAt = s.cfg.pillarEvery * (0.85 + s.rng() * 0.3);
  }
  s.pillars = s.pillars.filter(p => p.x + PILLAR_W > -8);
  // Collisions.
  const cy = s.copter.y;
  const sampAtCopter = sampleAt(s.samples, COPTER_X);
  if (sampAtCopter) {
    const topY = sampAtCopter.midY - sampAtCopter.gap / 2;
    const botY = sampAtCopter.midY + sampAtCopter.gap / 2;
    if (cy - COPTER_R < topY || cy + COPTER_R > botY) crash(s);
  }
  if (!s.over) {
    for (const p of s.pillars) {
      if (p.x + PILLAR_W < COPTER_X - COPTER_R) continue;
      if (p.x > COPTER_X + COPTER_R) continue;
      const gapTop = p.gapY - p.gapH / 2;
      const gapBot = p.gapY + p.gapH / 2;
      if (cy - COPTER_R < gapTop || cy + COPTER_R > gapBot) { crash(s); break; }
    }
  }
  // World ceiling/floor.
  if (cy < 8 || cy > VH - 8) crash(s);
  // Score.
  if (s.distance | 0 > s.score) s.score = s.distance | 0;
  s.score = s.distance | 0;
}

function crash(s) {
  s.copter.alive = false;
  s.over = true;
}

// Append a new sample on the right edge. The cave centre drifts toward a
// new target every ~5 samples and the gap interpolates toward gapMin as the
// run lengthens.
function addSample(s) {
  const xLast = s.samples.length ? s.samples[s.samples.length - 1].x : 0;
  const x = xLast + SAMPLE_DX;
  s.midDrift--;
  if (s.midDrift <= 0) {
    s.targetMidY = 80 + s.rng() * (VH - 160);
    s.midDrift = 5 + ((s.rng() * 4) | 0);
  }
  const k = 0.18;
  s.midY = s.midY * (1 - k) + s.targetMidY * k;
  // Shrink the gap over distance.
  const t = Math.min(1, s.distance / 5000);
  const gap = s.cfg.gap + (s.cfg.gapMin - s.cfg.gap) * t;
  s.samples.push({ x, midY: s.midY, gap });
}

function spawnPillar(s) {
  const gapH = 100;
  const samp = sampleAt(s.samples, VW + SAMPLE_DX);
  const gapY = samp ? samp.midY : VH / 2;
  s.pillars.push({ x: VW + 40, gapY, gapH });
}

// Linear interpolation of the cave sample at world x.
function sampleAt(samples, x) {
  if (!samples.length) return null;
  if (x <= samples[0].x) return samples[0];
  if (x >= samples[samples.length - 1].x) return samples[samples.length - 1];
  for (let i = 0; i < samples.length - 1; i++) {
    const a = samples[i], b = samples[i + 1];
    if (x >= a.x && x <= b.x) {
      const t = (x - a.x) / (b.x - a.x);
      return { x, midY: a.midY + (b.midY - a.midY) * t, gap: a.gap + (b.gap - a.gap) * t };
    }
  }
  return samples[samples.length - 1];
}
