// Pixel Lander - lunar-lander physics + per-level terrain.
//
// World units are the 360x480 canvas. Y grows downward; gravity is positive.
// The lander's `angle` is in radians, 0 = thrust straight UP. Thrust pushes
// the lander along -sin/+cos in world coords (positive angle -> right).

const VW = 360, VH = 480;

const GRAVITY      = 38;     // px / s^2
const THRUST_ACCEL = 110;    // px / s^2 while thrusting
const ROT_SPEED    = 2.4;    // rad / s
const FUEL_RATE    = 18;     // units / s while thrusting
const MAX_LAND_VY  = 28;     // soft-landing threshold (downward)
const MAX_LAND_VX  = 18;     // horizontal drift cap on landing
const MAX_LAND_TILT = 0.25;  // ±0.25 rad ≈ ±14° tilt cap on landing

const LANDER_W = 14, LANDER_H = 14;

// Each level: terrain as an array of {x,y} points across the full width plus
// a flat landing pad. `fuel` is the starting fuel budget. `wind` adds a
// constant horizontal acceleration.
const LEVELS = [
  { name: ['Sea of Calm', '静谧之海'], fuel: 1000, wind: 0,
    padX: 150, padW: 80,  padY: 420, terrain: 'gentle', seed: 31 },
  { name: ['Highlands', '高原'],       fuel: 900,  wind: 0,
    padX: 60,  padW: 60,  padY: 380, terrain: 'rolling', seed: 73 },
  { name: ['Crater Rim', '环形山'],    fuel: 800,  wind: 0,
    padX: 240, padW: 55,  padY: 360, terrain: 'craters', seed: 137 },
  { name: ['Cross Wind', '横风'],      fuel: 900,  wind: 10,
    padX: 130, padW: 60,  padY: 410, terrain: 'rolling', seed: 211 },
  { name: ['Narrow Shelf', '窄台'],    fuel: 750,  wind: -8,
    padX: 200, padW: 42,  padY: 340, terrain: 'craters', seed: 311 },
  { name: ['Final Descent', '终降'],   fuel: 700,  wind: 14,
    padX: 90,  padW: 38,  padY: 330, terrain: 'jagged', seed: 451 },
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

// Build the terrain polyline for a level. Points are sorted by x and the
// landing pad section is forced flat at `padY`.
function buildTerrain(cfg) {
  const rng = seededRandom(cfg.seed);
  const pts = [];
  // Sample heights every 24 px across the field.
  for (let x = 0; x <= VW; x += 18) {
    let baseY;
    if (cfg.terrain === 'gentle')   baseY = 410 + (rng() - 0.5) * 24;
    else if (cfg.terrain === 'rolling') baseY = 380 + Math.sin(x * 0.04) * 35 + (rng() - 0.5) * 18;
    else if (cfg.terrain === 'craters') baseY = 370 + Math.sin(x * 0.07) * 45 + (rng() - 0.5) * 24;
    else if (cfg.terrain === 'jagged')  baseY = 360 + (rng() - 0.5) * 80;
    else                                baseY = 400;
    if (baseY > 460) baseY = 460;
    if (baseY < 260) baseY = 260;
    pts.push({ x, y: baseY });
  }
  // Flatten the pad section.
  const padL = cfg.padX, padR = cfg.padX + cfg.padW;
  for (const p of pts) {
    if (p.x >= padL - 1 && p.x <= padR + 1) p.y = cfg.padY;
  }
  // Insert exact pad endpoints if they were skipped between samples.
  if (!pts.some(p => Math.abs(p.x - padL) < 1)) pts.push({ x: padL, y: cfg.padY });
  if (!pts.some(p => Math.abs(p.x - padR) < 1)) pts.push({ x: padR, y: cfg.padY });
  pts.sort((a, b) => a.x - b.x);
  return pts;
}

function buildGame(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const terrain = buildTerrain(cfg);
  return {
    levelIndex, cfg, terrain,
    lander: {
      x: VW / 2, y: 60,
      vx: (cfg.wind > 0 ? -8 : cfg.wind < 0 ? 8 : 0),   // tiny opposing drift
      vy: 0, angle: 0,
      alive: true, landed: false,
    },
    thrust: false, rotL: false, rotR: false,
    fuel: cfg.fuel,
    score: 0,
    over: false, won: false, started: false,
    tickCount: 0,
  };
}

// Return the terrain Y at world X by linear interpolation between samples.
function terrainYAt(terrain, x) {
  if (x <= terrain[0].x) return terrain[0].y;
  if (x >= terrain[terrain.length - 1].x) return terrain[terrain.length - 1].y;
  for (let i = 0; i < terrain.length - 1; i++) {
    const a = terrain[i], b = terrain[i + 1];
    if (x >= a.x && x <= b.x) {
      const t = (x - a.x) / (b.x - a.x);
      return a.y + (b.y - a.y) * t;
    }
  }
  return terrain[terrain.length - 1].y;
}

// `dt` in seconds. Caller passes input flags via setInputs() before tick.
function tick(s, dt) {
  if (s.over) return;
  if (!s.started) return;
  s.tickCount = (s.tickCount || 0) + 1;
  const L = s.lander;
  if (!L.alive) {
    // Brief free-fall after a crash before the result banner.
    L.vy += GRAVITY * dt;
    L.y  += L.vy * dt;
    if (s.tickCount > 60) { s.over = true; s.won = false; }
    return;
  }
  if (L.landed) {
    s.over = true; s.won = true;
    s.score = Math.max(100, ((s.fuel | 0) * 2) + 200);
    return;
  }
  // Rotation.
  if (s.rotL) L.angle -= ROT_SPEED * dt;
  if (s.rotR) L.angle += ROT_SPEED * dt;
  // Thrust along -sin/+cos (angle 0 -> straight up).
  if (s.thrust && s.fuel > 0) {
    L.vx += Math.sin(L.angle) * THRUST_ACCEL * dt;
    L.vy -= Math.cos(L.angle) * THRUST_ACCEL * dt;
    s.fuel -= FUEL_RATE * dt;
    if (s.fuel < 0) s.fuel = 0;
  }
  // Gravity + wind.
  L.vy += GRAVITY * dt;
  L.vx += s.cfg.wind * dt;
  // Position.
  L.x += L.vx * dt;
  L.y += L.vy * dt;
  // Drift off the world.
  if (L.x < -LANDER_W || L.x > VW + LANDER_W || L.y < -120) {
    L.alive = false;
    return;
  }
  // Terrain collision: check the bottom-centre and the two skids.
  const skids = [L.x - 6, L.x, L.x + 6];
  for (const sx of skids) {
    const ty = terrainYAt(s.terrain, sx);
    if (L.y + LANDER_H / 2 >= ty) {
      // On a pad?
      const pad = s.cfg;
      const onPad = (sx >= pad.padX - 2 && sx <= pad.padX + pad.padW + 2)
                  && Math.abs(terrainYAt(s.terrain, sx) - pad.padY) < 1;
      if (onPad
          && L.vy < MAX_LAND_VY
          && Math.abs(L.vx) < MAX_LAND_VX
          && Math.abs(L.angle) < MAX_LAND_TILT) {
        L.landed = true;
        L.y = pad.padY - LANDER_H / 2;
        L.vx = 0; L.vy = 0;
      } else {
        L.alive = false;
      }
      return;
    }
  }
}

function setInputs(s, { thrust, rotL, rotR }) {
  s.thrust = !!thrust;
  s.rotL = !!rotL;
  s.rotR = !!rotR;
  if (!s.started && (thrust || rotL || rotR)) s.started = true;
}

function startGame(s) { s.started = true; }
