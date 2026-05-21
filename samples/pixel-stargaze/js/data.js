// Pixel Stargaze - an idle astronomy game. Pure economy logic.
//
// Collect Light by tapping the night sky and by running telescopes that
// gather it passively. Spend Light on more telescopes and on research that
// multiplies output. When a run has gathered enough, Publish a paper to reset
// and bank Renown - a permanent global multiplier. Idle gains accrue offline.

const VW = 360, VH = 480;

const TAP_BASE = 1;
const COST_MULT = 1.15;            // telescope price growth per unit bought
const OFFLINE_CAP = 8 * 3600;      // seconds of offline catch-up granted
const PUBLISH_MIN = 100000;        // lifetime Light needed to publish a paper
const PUBLISH_SCALE = 12000;       // renown = floor(sqrt(lifetime / scale))
const RENOWN_BONUS = 0.15;         // each renown adds +15% to all output

// telescope tiers: a unit produces `output` Light/sec; price grows per unit.
const TELESCOPES = [
  { key: 'lens',   name: ['Lens', '透镜'],       cost: 12,      output: 0.25 },
  { key: 'reflic', name: ['Reflector', '反射镜'], cost: 130,     output: 2.2 },
  { key: 'astro',  name: ['Astrograph', '天图仪'], cost: 1500,    output: 17 },
  { key: 'dish',   name: ['Radio Dish', '射电盘'], cost: 18000,   output: 130 },
  { key: 'scope',  name: ['Space Scope', '太空镜'], cost: 220000,  output: 950 },
];
const TELESCOPE_COUNT = TELESCOPES.length;

// one-time research. target: 'all' | 'tap' | a telescope index.
const RESEARCH = [
  { key: 'glass',  name: ['Ground Glass', '研磨镜片'],   cost: 60,      target: 0,     mult: 2.5 },
  { key: 'hands',  name: ['Steady Hands', '稳健之手'],   cost: 150,     target: 'tap', mult: 4 },
  { key: 'coat',   name: ['Coated Optics', '镀膜光学'],  cost: 1100,    target: 1,     mult: 2.5 },
  { key: 'adapt',  name: ['Adaptive Optics', '自适应光学'], cost: 9000,  target: 'all', mult: 2 },
  { key: 'plate',  name: ['Plate Archive', '底片档案'],   cost: 75000,   target: 2,     mult: 3 },
  { key: 'synth',  name: ['Aperture Synthesis', '综合孔径'], cost: 600000, target: 'all', mult: 2 },
  { key: 'cryo',   name: ['Cryo Receivers', '低温接收机'], cost: 4500000, target: 3,     mult: 3 },
  { key: 'darksky', name: ['Dark Sky Site', '暗夜台址'],   cost: 30000000, target: 'all', mult: 2 },
];

function newGame() {
  return {
    light: 0,
    lifetime: 0,                   // Light earned this run (for publishing)
    totalRenown: 0,
    scopes: new Array(TELESCOPE_COUNT).fill(0),
    research: {},                  // key -> true
    taps: 0,
  };
}

// ---- derived values ------------------------------------------------------
function globalMult(s) {
  let m = 1 + RENOWN_BONUS * s.totalRenown;
  for (const r of RESEARCH) if (s.research[r.key] && r.target === 'all') m *= r.mult;
  return m;
}
function tierMult(s, tier) {
  let m = 1;
  for (const r of RESEARCH) if (s.research[r.key] && r.target === tier) m *= r.mult;
  return m;
}
function tapValue(s) {
  let m = 1;
  for (const r of RESEARCH) if (s.research[r.key] && r.target === 'tap') m *= r.mult;
  return TAP_BASE * m * globalMult(s);
}
function rate(s) {
  let total = 0;
  for (let i = 0; i < TELESCOPE_COUNT; i++) {
    total += s.scopes[i] * TELESCOPES[i].output * tierMult(s, i);
  }
  return total * globalMult(s);
}
function scopeCost(s, tier) {
  return Math.ceil(TELESCOPES[tier].cost * Math.pow(COST_MULT, s.scopes[tier]));
}

// ---- actions -------------------------------------------------------------
function tap(s) {
  const v = tapValue(s);
  s.light += v;
  s.lifetime += v;
  s.taps++;
  return v;
}
function tick(s, dt) {
  const g = rate(s) * dt;
  s.light += g;
  s.lifetime += g;
  return g;
}
function buyScope(s, tier) {
  const c = scopeCost(s, tier);
  if (s.light < c) return false;
  s.light -= c;
  s.scopes[tier]++;
  return true;
}
function researchBought(s, key) { return s.research[key] === true; }
function buyResearch(s, key) {
  if (s.research[key]) return false;
  const r = RESEARCH.find(x => x.key === key);
  if (!r || s.light < r.cost) return false;
  s.light -= r.cost;
  s.research[key] = true;
  return true;
}

function canPublish(s) { return s.lifetime >= PUBLISH_MIN; }
function renownFor(s) { return Math.floor(Math.sqrt(s.lifetime / PUBLISH_SCALE)); }
function publish(s) {
  if (!canPublish(s)) return 0;
  const gain = renownFor(s);
  s.totalRenown += gain;
  s.light = 0;
  s.lifetime = 0;
  s.scopes = new Array(TELESCOPE_COUNT).fill(0);
  s.research = {};
  return gain;
}

// award capped offline Light for `seconds` away; returns the amount granted
function applyOffline(s, seconds) {
  const sec = Math.max(0, Math.min(OFFLINE_CAP, seconds));
  const g = rate(s) * sec;
  s.light += g;
  s.lifetime += g;
  return g;
}
