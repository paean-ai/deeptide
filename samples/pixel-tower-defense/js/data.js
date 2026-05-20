// Pixel Tower Defense - game data: towers, enemies, levels, waves

const TILE = 40;
const GRID_W = 20;
const GRID_H = 15;
const FIELD_W = GRID_W * TILE; // 800
const FIELD_H = GRID_H * TILE; // 600

// --- Towers -------------------------------------------------------------
// Each tower: base stats + tier2 + two tier3 branches.
// element: 'physical' | 'magic'  -- mode: 'projectile' | 'beam' | 'aura'
const TOWERS = {
  arrow: {
    id: 'arrow', element: 'physical', mode: 'projectile', air: true,
    color: '#7fe8ff', accent: '#3a7fa8', baseCost: 70,
    tiers: [
      { cost: 70, damage: 15, range: 112, interval: 0.55, projSpeed: 460, crit: 0.05 },
      { cost: 95, damage: 30, range: 132, interval: 0.50, projSpeed: 500, crit: 0.10 },
    ],
    branches: {
      sniper: { cost: 175, damage: 96, range: 250, interval: 1.05, projSpeed: 900, crit: 0.40, critMult: 2.4 },
      rapid: { cost: 175, damage: 26, range: 146, interval: 0.18, projSpeed: 560, crit: 0.12 },
    },
  },
  cannon: {
    id: 'cannon', element: 'physical', mode: 'projectile', air: false,
    color: '#f4c85a', accent: '#8a6526', baseCost: 100,
    tiers: [
      { cost: 100, damage: 42, range: 98, interval: 1.30, projSpeed: 320, splash: 50 },
      { cost: 135, damage: 78, range: 110, interval: 1.20, projSpeed: 340, splash: 60 },
    ],
    branches: {
      mortar: { cost: 240, damage: 96, range: 230, interval: 2.15, projSpeed: 260, splash: 96 },
      demolisher: { cost: 240, damage: 230, range: 116, interval: 1.55, projSpeed: 360, splash: 44, shred: 7 },
    },
  },
  frost: {
    id: 'frost', element: 'magic', mode: 'beam', air: true,
    color: '#9ceaff', accent: '#3f7d96', baseCost: 85,
    tiers: [
      { cost: 85, damage: 8, range: 104, interval: 0.80, slow: 0.35, slowDur: 1.3 },
      { cost: 115, damage: 17, range: 118, interval: 0.72, slow: 0.45, slowDur: 1.5 },
    ],
    branches: {
      glacier: { cost: 200, damage: 32, range: 126, interval: 0.70, slow: 0.55, slowDur: 1.6, freezeChance: 0.28, freezeDur: 0.9 },
      blizzard: { cost: 200, damage: 24, range: 140, interval: 0.62, slow: 0.5, slowDur: 0.9, mode: 'aura' },
    },
  },
  arcane: {
    id: 'arcane', element: 'magic', mode: 'beam', air: true,
    color: '#c69bff', accent: '#6a4a9a', baseCost: 120,
    tiers: [
      { cost: 120, damage: 22, range: 116, interval: 0.90, chain: 2, chainRange: 90 },
      { cost: 155, damage: 42, range: 132, interval: 0.82, chain: 3, chainRange: 104 },
    ],
    branches: {
      storm: { cost: 270, damage: 50, range: 154, interval: 0.78, chain: 6, chainRange: 128 },
      tesla: { cost: 270, damage: 168, range: 134, interval: 0.70, chain: 1, chainRange: 80 },
    },
  },
};

// Build the resolved stat block for a placed tower.
function towerStats(type, tier, branch) {
  const def = TOWERS[type];
  let s;
  if (tier <= 2) s = { ...def.tiers[tier - 1] };
  else s = { ...def.branches[branch] };
  s.element = def.element;
  s.air = def.air;
  s.mode = s.mode || def.mode;
  return s;
}

// Total gold invested into a tower at a given tier (for sell value).
function towerInvested(type, tier, branch) {
  const def = TOWERS[type];
  let total = def.tiers[0].cost;
  if (tier >= 2) total += def.tiers[1].cost;
  if (tier >= 3) total += def.branches[branch].cost;
  return total;
}

// --- Enemies ------------------------------------------------------------
const ENEMIES = {
  grunt:   { hp: 62,  speed: 54, reward: 6,  armor: 0,  resist: 0,    air: false, size: 26, leak: 1 },
  runner:  { hp: 34,  speed: 104, reward: 5, armor: 0,  resist: 0,    air: false, size: 22, leak: 1 },
  swarm:   { hp: 19,  speed: 70, reward: 3,  armor: 0,  resist: 0,    air: false, size: 17, leak: 1 },
  armored: { hp: 168, speed: 40, reward: 13, armor: 9,  resist: 0.12, air: false, size: 30, leak: 1 },
  flyer:   { hp: 74,  speed: 76, reward: 9,  armor: 0,  resist: 0,    air: true,  size: 24, leak: 1 },
  healer:  { hp: 118, speed: 48, reward: 15, armor: 2,  resist: 0.1,  air: false, size: 27, leak: 1, healRate: 14, healRange: 86 },
  boss:    { hp: 1500, speed: 30, reward: 130, armor: 7, resist: 0.28, air: false, size: 46, leak: 8 },
};

// --- Levels -------------------------------------------------------------
// path: tile coordinates (may go off-grid at ends). decos: blocked tiles.
const LEVELS = [
  {
    id: 0, name: { en: 'Greenfield', zh: '青野平原' }, theme: 'grass',
    startGold: 250, startLives: 20, totalWaves: 14,
    path: [[-1,3],[4,3],[4,10],[10,10],[10,3],[16,3],[16,11],[20,11]],
    decos: [[7,5],[8,5],[13,7],[2,12],[17,6],[6,1]],
  },
  {
    id: 1, name: { en: 'Frostpeak', zh: '霜峰隘口' }, theme: 'snow',
    startGold: 280, startLives: 18, totalWaves: 16,
    path: [[-1,7],[3,7],[3,2],[8,2],[8,12],[13,12],[13,5],[17,5],[17,12],[20,12]],
    decos: [[5,5],[10,8],[15,9],[6,10],[11,2],[1,11],[18,3]],
  },
  {
    id: 2, name: { en: 'Emberforge', zh: '熔火炼狱' }, theme: 'lava',
    startGold: 300, startLives: 16, totalWaves: 18,
    path: [[10,-1],[10,4],[3,4],[3,10],[16,10],[16,3],[12,3],[12,7],[7,7],[7,12],[20,12]],
    decos: [[5,6],[14,6],[9,9],[1,2],[18,8],[5,1],[14,1]],
  },
  {
    id: 3, name: { en: 'Glacier Maze', zh: '冰川迷宫' }, theme: 'snow',
    startGold: 320, startLives: 16, totalWaves: 20,
    path: [[-1,2],[6,2],[6,8],[2,8],[2,12],[14,12],[14,5],[9,5],[9,9],[17,9],[17,2],[20,2]],
    decos: [[4,5],[10,2],[5,10],[11,8],[16,12],[8,3],[12,3]],
  },
  {
    id: 4, name: { en: 'Cinder Spiral', zh: '余烬螺旋' }, theme: 'lava',
    startGold: 340, startLives: 15, totalWaves: 22,
    path: [[3,-1],[3,5],[16,5],[16,9],[6,9],[6,13],[12,13],[12,2],[19,2],[19,13],[20,13]],
    decos: [[8,3],[10,7],[14,11],[2,8],[9,4],[5,2]],
  },
  {
    id: 5, name: { en: 'Verdant Coil', zh: '碧野回廊' }, theme: 'grass',
    startGold: 360, startLives: 14, totalWaves: 24,
    path: [[-1,2],[4,2],[4,12],[9,12],[9,4],[14,4],[14,12],[18,12],[18,2],[20,2]],
    decos: [[2,7],[7,7],[12,8],[16,7],[6,5],[11,10]],
  },
  {
    id: 6, name: { en: 'Magma Switchback', zh: '熔岩折径' }, theme: 'lava',
    startGold: 380, startLives: 13, totalWaves: 26,
    path: [[-1,12],[3,12],[3,3],[7,3],[7,11],[11,11],[11,3],[15,3],[15,12],[18,12],[18,5],[20,5]],
    decos: [[5,7],[9,7],[13,7],[5,1],[13,9],[9,14]],
  },
  {
    id: 7, name: { en: 'Frostbite Coil', zh: '霜噬回环' }, theme: 'snow',
    startGold: 400, startLives: 12, totalWaves: 28,
    path: [[-1,2],[5,2],[5,6],[2,6],[2,11],[8,11],[8,4],[12,4],[12,11],[16,11],[16,4],[19,4],[19,13],[20,13]],
    decos: [[10,2],[14,7],[6,9],[10,8],[14,2],[3,9]],
  },
  {
    id: 8, name: { en: 'Meadow Maze', zh: '草甸迷阵' }, theme: 'grass',
    startGold: 420, startLives: 11, totalWaves: 30,
    path: [[-1,7],[4,7],[4,2],[9,2],[9,12],[13,12],[13,4],[17,4],[17,11],[20,11]],
    decos: [[6,9],[11,7],[15,8],[2,10],[7,4],[15,2]],
  },
  {
    id: 9, name: { en: 'Tundra Coil', zh: '冻原回廊' }, theme: 'snow',
    startGold: 440, startLives: 10, totalWaves: 32,
    path: [[-1,5],[3,5],[3,1],[7,1],[7,8],[11,8],[11,2],[15,2],[15,11],[19,11],[19,5],[20,5]],
    decos: [[5,3],[9,5],[13,7],[17,7],[2,10],[10,12]],
  },
];

// --- Wave generation ----------------------------------------------------
// Each wave -> list of groups: { type, count, interval, delay }
function buildWaves(level) {
  const waves = [];
  const total = level.totalWaves;
  const diff = 1 + level.id * 0.35;
  for (let w = 1; w <= total; w++) {
    const groups = [];
    const boss = w % 5 === 0;
    if (boss) {
      // softeners then a boss
      groups.push({ type: 'swarm', count: 8 + w, interval: 0.3, delay: 0 });
      if (w >= 10) groups.push({ type: 'armored', count: 2 + Math.floor(w / 5), interval: 1.0, delay: 1.5 });
      groups.push({ type: 'boss', count: w >= 15 ? 2 : 1, interval: 3.5, delay: 3 });
    } else {
      groups.push({ type: 'grunt', count: 4 + w, interval: 0.7, delay: 0 });
      if (w >= 2) groups.push({ type: 'runner', count: 2 + Math.floor(w * 0.9), interval: 0.4, delay: 1.2 });
      if (w >= 3 && w % 2 === 1) groups.push({ type: 'swarm', count: 5 + w * 2, interval: 0.22, delay: 2 });
      if (w >= 4) groups.push({ type: 'armored', count: 1 + Math.floor(w / 3), interval: 1.1, delay: 1.5 });
      if (w >= 6) groups.push({ type: 'flyer', count: 2 + Math.floor(w / 2), interval: 0.6, delay: 2.5 });
      if (w >= 8 && w % 3 === 0) groups.push({ type: 'healer', count: 1 + Math.floor(w / 8), interval: 1.5, delay: 1 });
    }
    // hp multiplier ramps with wave + level difficulty
    const hpMul = (1 + (w - 1) * 0.16) * diff;
    waves.push({ groups, hpMul, boss });
  }
  return waves;
}

// Endless wave beyond totalWaves (escalating).
function buildEndlessWave(w, level) {
  const diff = 1 + level.id * 0.35;
  const groups = [
    { type: 'grunt', count: 8 + w, interval: 0.4, delay: 0 },
    { type: 'runner', count: 6 + w, interval: 0.3, delay: 1 },
    { type: 'armored', count: 3 + Math.floor(w / 2), interval: 0.9, delay: 1.5 },
    { type: 'flyer', count: 4 + Math.floor(w / 2), interval: 0.5, delay: 2 },
  ];
  if (w % 3 === 0) groups.push({ type: 'boss', count: 1 + Math.floor(w / 6), interval: 2.5, delay: 3 });
  if (w % 2 === 0) groups.push({ type: 'healer', count: 2, interval: 1.2, delay: 1 });
  const hpMul = (2.6 + w * 0.42) * diff;
  return { groups, hpMul, boss: w % 3 === 0 };
}
