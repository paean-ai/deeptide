// Pixel Bastion - a Missile Command-style city defender. Tap the sky to
// launch a counter-missile - the closest silo with ammo fires it.

const VW = 360, VH = 480;
const GROUND_Y = VH - 56;            // 424
const CITY_W = 24, CITY_H = 22;
const CITY_X = [50, 110, 170, 230, 290];
const SILOS = [{ x: 22, y: GROUND_Y + 8 }, { x: 180, y: GROUND_Y + 8 }, { x: 338, y: GROUND_Y + 8 }];

const SILO_AMMO = 30;
const COUNTER_SPEED = 300;
const EXPLOSION_R = 28;
const EXPLOSION_LIFE = 0.85;
const GROW_TIME = 0.28;
const WAVE_END_PAUSE = 0.6;

// `splitChance` is the chance a given incoming missile is a MIRV that
// splits into two fresh warheads partway down - kill it before it does.
const LEVELS = [
  { name: ['First Wave', '首波'],   seed: 11,  count: 8,  speed: 50, spawn: 1.6,  splitChance: 0 },
  { name: ['Skywatch', '巡天'],     seed: 41,  count: 12, speed: 60, spawn: 1.4,  splitChance: 0 },
  { name: ['Crimson Storm', '红潮'], seed: 89,  count: 16, speed: 70, spawn: 1.2,  splitChance: 0 },
  { name: ['Sky Siege', '天围'],    seed: 156, count: 20, speed: 80, spawn: 1.05, splitChance: 0 },
  { name: ['Inferno', '炼狱'],      seed: 244, count: 25, speed: 88, spawn: 0.9,  splitChance: 0 },
  { name: ['Last Stand', '死守'],   seed: 357, count: 26, speed: 90, spawn: 0.88, splitChance: 0 },
  { name: ['MIRV Rain', '裂变雨'],  seed: 471, count: 24, speed: 84, spawn: 1.0,  splitChance: 0.28 },
  { name: ['Hydra Sky', '九头空'],  seed: 588, count: 27, speed: 90, spawn: 0.92, splitChance: 0.42 },
  { name: ['Armageddon', '末日'],   seed: 701, count: 30, speed: 95, spawn: 0.84, splitChance: 0.55 },
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
  return {
    levelIndex, cfg,
    rng: seededRandom(cfg.seed),
    silos: SILOS.map(s => ({ x: s.x, y: s.y, ammo: SILO_AMMO })),
    cities: CITY_X.map(x => ({ x, y: GROUND_Y, alive: true })),
    incoming: [], counters: [], explosions: [],
    spawned: 0, downed: 0,
    spawnTimer: 0.6,
    waveEndT: 0,
    score: 0, over: false, won: false,
  };
}

// ---- spawn ---------------------------------------------------------------
function spawnIncoming(s) {
  if (s.spawned >= s.cfg.count) return;
  const sx = 14 + s.rng() * (VW - 28);
  let tx;
  if (s.rng() < 0.75) {                                // most aimed at a city
    tx = CITY_X[(s.rng() * CITY_X.length) | 0] + (s.rng() - 0.5) * 12;
  } else {
    tx = 20 + s.rng() * (VW - 40);
  }
  const dx = tx - sx, dy = GROUND_Y;
  const dist = Math.hypot(dx, dy);
  const v = s.cfg.speed * (0.85 + s.rng() * 0.3);
  // Some missiles are MIRVs that split partway down.
  const isMirv = s.rng() < (s.cfg.splitChance || 0);
  s.incoming.push({
    sx, sy: 0, x: sx, y: 0, tx, ty: GROUND_Y,
    vx: dx / dist * v, vy: dy / dist * v, alive: true,
    split: isMirv ? 2 : 0,
    splitY: isMirv ? 150 + s.rng() * 100 : 0,
  });
  s.spawned++;
}

// A MIRV warhead spawned mid-air when its parent splits. Aimed afresh at
// a city; it never splits again.
function spawnChild(s, x, y) {
  const tx = CITY_X[(s.rng() * CITY_X.length) | 0] + (s.rng() - 0.5) * 16;
  const dx = tx - x, dy = GROUND_Y - y;
  const dist = Math.hypot(dx, dy) || 1;
  const v = s.cfg.speed * (0.9 + s.rng() * 0.3);
  return {
    sx: x, sy: y, x, y, tx, ty: GROUND_Y,
    vx: dx / dist * v, vy: dy / dist * v, alive: true,
    split: 0, splitY: 0,
  };
}

// ---- player action ------------------------------------------------------
function fireCounter(s, tx, ty) {
  if (s.over) return false;
  if (ty >= GROUND_Y - 4) return false;                // must aim above ground
  let best = null, bd = 1e9;
  for (const silo of s.silos) {
    if (silo.ammo <= 0) continue;
    const d = Math.abs(silo.x - tx);
    if (d < bd) { bd = d; best = silo; }
  }
  if (!best) return false;
  best.ammo--;
  s.counters.push({ sx: best.x, sy: best.y, x: best.x, y: best.y, tx, ty, done: false });
  return true;
}

// ---- world tick ----------------------------------------------------------
function tick(s, dt) {
  if (s.over) return;
  // spawns
  s.spawnTimer -= dt;
  if (s.spawnTimer <= 0 && s.spawned < s.cfg.count) {
    spawnIncoming(s);
    s.spawnTimer = s.cfg.spawn * (0.7 + s.rng() * 0.6);
  }
  // move incoming
  const children = [];
  for (const m of s.incoming) {
    if (!m.alive) continue;
    m.x += m.vx * dt; m.y += m.vy * dt;
    if (m.split > 0 && m.y >= m.splitY) {
      // MIRV splits: the parent is consumed, two warheads take its place.
      for (let k = 0; k < m.split; k++) children.push(spawnChild(s, m.x, m.y));
      m.alive = false;
      continue;
    }
    if (m.y >= GROUND_Y) {
      m.alive = false;
      damageGround(s, m.x);
    }
  }
  for (const c of children) s.incoming.push(c);
  // move counter-missiles
  for (const c of s.counters) {
    if (c.done) continue;
    const dx = c.tx - c.x, dy = c.ty - c.y;
    const dist = Math.hypot(dx, dy);
    if (dist <= COUNTER_SPEED * dt) {
      c.x = c.tx; c.y = c.ty; c.done = true;
      s.explosions.push({ x: c.tx, y: c.ty, age: 0 });
    } else {
      c.x += dx / dist * COUNTER_SPEED * dt;
      c.y += dy / dist * COUNTER_SPEED * dt;
    }
  }
  // explosions evolve, destroy incoming inside the blast
  for (const e of s.explosions) {
    e.age += dt;
    const r = explosionRadius(e.age);
    if (r <= 0) continue;
    for (const m of s.incoming) {
      if (!m.alive) continue;
      const dx = m.x - e.x, dy = m.y - e.y;
      if (dx * dx + dy * dy <= r * r) {
        m.alive = false; s.downed++; s.score += 10;
      }
    }
  }
  // cleanup
  s.incoming = s.incoming.filter(m => m.alive);
  s.counters = s.counters.filter(c => !c.done);
  s.explosions = s.explosions.filter(e => e.age < EXPLOSION_LIFE);
  // wave end / lose
  if (!s.cities.some(c => c.alive)) {
    s.over = true; s.won = false;
    return;
  }
  if (s.spawned >= s.cfg.count && s.incoming.length === 0 && s.counters.length === 0 && s.explosions.length === 0) {
    s.waveEndT += dt;
    if (s.waveEndT >= WAVE_END_PAUSE) {
      s.over = true; s.won = true;
      s.score += s.cities.filter(c => c.alive).length * 50;
    }
  }
}

function explosionRadius(age) {
  if (age < GROW_TIME) return EXPLOSION_R * (age / GROW_TIME);
  if (age < EXPLOSION_LIFE) return EXPLOSION_R * (1 - (age - GROW_TIME) / (EXPLOSION_LIFE - GROW_TIME));
  return 0;
}

function damageGround(s, x) {
  for (const c of s.cities) {
    if (!c.alive) continue;
    if (Math.abs(c.x - x) <= CITY_W / 2 + 6) { c.alive = false; return; }
  }
}
