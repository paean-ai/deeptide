// Pixel Survivors - game data: weapons, passives, enemies, meta upgrades

// ---- Weapons -----------------------------------------------------------
// tiers index 0..5 = levels 1..6 (level 6 = evolved form)
const WEAPONS = {
  dagger: {
    icon: '🗡', color: '#cfe8ff',
    tiers: [
      { dmg: 11, count: 1, cd: 0.55, pierce: 1, speed: 360 },
      { dmg: 15, count: 1, cd: 0.50, pierce: 1, speed: 380 },
      { dmg: 19, count: 2, cd: 0.48, pierce: 1, speed: 400 },
      { dmg: 25, count: 2, cd: 0.42, pierce: 2, speed: 420 },
      { dmg: 32, count: 3, cd: 0.38, pierce: 2, speed: 440 },
      { dmg: 44, count: 5, cd: 0.30, pierce: 3, speed: 480 },
    ],
  },
  aura: {
    icon: '✺', color: '#9cffd0',
    tiers: [
      { dmg: 5, radius: 52, cd: 0.5 },
      { dmg: 7, radius: 58, cd: 0.5 },
      { dmg: 9, radius: 66, cd: 0.45 },
      { dmg: 12, radius: 74, cd: 0.45 },
      { dmg: 16, radius: 84, cd: 0.4 },
      { dmg: 24, radius: 104, cd: 0.32 },
    ],
  },
  orbit: {
    icon: '◓', color: '#ffd34d',
    tiers: [
      { dmg: 13, count: 1, radius: 56, speed: 2.6 },
      { dmg: 16, count: 2, radius: 60, speed: 2.8 },
      { dmg: 20, count: 2, radius: 66, speed: 3.0 },
      { dmg: 26, count: 3, radius: 72, speed: 3.2 },
      { dmg: 33, count: 4, radius: 80, speed: 3.5 },
      { dmg: 46, count: 6, radius: 92, speed: 4.0 },
    ],
  },
  bolt: {
    icon: '⚡', color: '#bda6ff',
    tiers: [
      { dmg: 20, count: 1, cd: 1.5 },
      { dmg: 26, count: 1, cd: 1.35 },
      { dmg: 33, count: 2, cd: 1.25 },
      { dmg: 43, count: 2, cd: 1.1 },
      { dmg: 56, count: 3, cd: 0.95 },
      { dmg: 78, count: 5, cd: 0.8 },
    ],
  },
  nova: {
    icon: '❄', color: '#9ce6ff',
    tiers: [
      { dmg: 9, radius: 92, cd: 2.6, slow: 0.4 },
      { dmg: 13, radius: 102, cd: 2.4, slow: 0.42 },
      { dmg: 18, radius: 114, cd: 2.2, slow: 0.45 },
      { dmg: 24, radius: 128, cd: 2.0, slow: 0.48 },
      { dmg: 32, radius: 144, cd: 1.8, slow: 0.52 },
      { dmg: 48, radius: 176, cd: 1.5, slow: 0.6 },
    ],
  },
  fireball: {
    icon: '🔥', color: '#ff9c4d',
    tiers: [
      { dmg: 24, count: 1, splash: 42, cd: 1.6, speed: 240 },
      { dmg: 32, count: 1, splash: 46, cd: 1.5, speed: 250 },
      { dmg: 42, count: 1, splash: 52, cd: 1.4, speed: 260 },
      { dmg: 56, count: 2, splash: 58, cd: 1.3, speed: 270 },
      { dmg: 74, count: 2, splash: 66, cd: 1.15, speed: 285 },
      { dmg: 104, count: 3, splash: 84, cd: 0.95, speed: 320 },
    ],
  },
  shards: {
    icon: '✶', color: '#ff8fd0',
    tiers: [
      { dmg: 8,  count: 4,  cd: 0.95, pierce: 1, speed: 300 },
      { dmg: 10, count: 5,  cd: 0.90, pierce: 1, speed: 310 },
      { dmg: 13, count: 6,  cd: 0.82, pierce: 1, speed: 320 },
      { dmg: 17, count: 8,  cd: 0.74, pierce: 2, speed: 340 },
      { dmg: 22, count: 10, cd: 0.66, pierce: 2, speed: 360 },
      { dmg: 32, count: 16, cd: 0.52, pierce: 3, speed: 400 },
    ],
  },
  boomerang: {
    icon: '🪃', color: '#7ee0c4',
    tiers: [
      { dmg: 14, count: 1, cd: 1.25, speed: 320, out: 0.42 },
      { dmg: 18, count: 1, cd: 1.15, speed: 335, out: 0.45 },
      { dmg: 23, count: 2, cd: 1.05, speed: 350, out: 0.48 },
      { dmg: 30, count: 2, cd: 0.95, speed: 365, out: 0.52 },
      { dmg: 39, count: 3, cd: 0.85, speed: 385, out: 0.55 },
      { dmg: 56, count: 4, cd: 0.68, speed: 420, out: 0.60 },
    ],
  },
  coil: {
    icon: '↯', color: '#ffe04a',
    tiers: [
      // count = number of chain jumps, range = max jump distance
      { dmg: 14, count: 2, cd: 1.30, range: 110 },
      { dmg: 18, count: 3, cd: 1.20, range: 120 },
      { dmg: 23, count: 4, cd: 1.10, range: 130 },
      { dmg: 30, count: 5, cd: 1.00, range: 145 },
      { dmg: 39, count: 6, cd: 0.90, range: 160 },
      { dmg: 56, count: 9, cd: 0.70, range: 195 },
    ],
  },
  skyfall: {
    icon: '☄', color: '#ffd24a',
    tiers: [
      { dmg: 30,  count: 2, splash: 46, cd: 2.0,  speed: 300 },
      { dmg: 39,  count: 2, splash: 50, cd: 1.85, speed: 310 },
      { dmg: 50,  count: 3, splash: 56, cd: 1.7,  speed: 320 },
      { dmg: 64,  count: 3, splash: 62, cd: 1.5,  speed: 335 },
      { dmg: 82,  count: 4, splash: 70, cd: 1.3,  speed: 355 },
      { dmg: 116, count: 6, splash: 88, cd: 1.05, speed: 390 },
    ],
  },
};
const WEAPON_IDS = Object.keys(WEAPONS);

// ---- Passive items -----------------------------------------------------
const PASSIVES = {
  might:   { icon: '💪', max: 5 },  // +9% damage / level
  swift:   { icon: '👟', max: 5 },  // +8% move speed / level
  haste:   { icon: '⏩', max: 5 },  // -8% cooldown / level
  armor:   { icon: '🛡', max: 5 },  // -1.6 contact damage / level
  magnet:  { icon: '🧲', max: 5 },  // +30% pickup range / level
  vitality:{ icon: '❤', max: 5 },  // +24 max HP / level
};
const PASSIVE_IDS = Object.keys(PASSIVES);

function passiveBonus(id, lvl) {
  switch (id) {
    case 'might': return lvl * 0.09;
    case 'swift': return lvl * 0.08;
    case 'haste': return lvl * 0.08;
    case 'armor': return lvl * 1.6;
    case 'magnet': return lvl * 0.30;
    case 'vitality': return lvl * 24;
    default: return 0;
  }
}

// ---- Enemies -----------------------------------------------------------
const ENEMIES = {
  bat:     { hp: 13, speed: 64, dmg: 6,  xp: 1, size: 16, color: '#9a7ad8', sprite: 'bat' },
  skel:    { hp: 21, speed: 50, dmg: 8,  xp: 1, size: 19, color: '#dfe4ee', sprite: 'skel' },
  zombie:  { hp: 32, speed: 34, dmg: 10, xp: 2, size: 21, color: '#6fae3e', sprite: 'zombie' },
  ghost:   { hp: 18, speed: 58, dmg: 7,  xp: 2, size: 19, color: '#bcd4e8', sprite: 'ghost' },
  slime:   { hp: 50, speed: 27, dmg: 11, xp: 3, size: 23, color: '#5fc7d8', sprite: 'slime' },
  brute:   { hp: 96, speed: 31, dmg: 17, xp: 6, size: 30, color: '#c0392b', sprite: 'brute' },
};
const BOSSES = {
  warden:  { hp: 1100, speed: 38, dmg: 24, xp: 60, size: 52, color: '#b03a4a', sprite: 'brute', boss: true },
  reaper:  { hp: 1700, speed: 44, dmg: 30, xp: 90, size: 58, color: '#7d4fbe', sprite: 'ghost', boss: true },
  overlord:{ hp: 5200, speed: 40, dmg: 38, xp: 300, size: 72, color: '#ff5a3a', sprite: 'brute', boss: true },
};

// time-based difficulty: returns a spawn profile for elapsed minutes
function spawnProfile(minutes) {
  const tier = Math.min(5, Math.floor(minutes / 2.5));
  const pools = [
    ['bat'],
    ['bat', 'skel'],
    ['bat', 'skel', 'zombie'],
    ['skel', 'zombie', 'ghost'],
    ['zombie', 'ghost', 'slime'],
    ['ghost', 'slime', 'brute'],
  ];
  return {
    pool: pools[tier],
    interval: Math.max(0.22, 1.05 - minutes * 0.075),
    batch: 1 + Math.floor(minutes / 2),
    hpMul: 1 + minutes * 0.42,
    speedMul: 1 + minutes * 0.018,
  };
}

// ---- Meta upgrades (persist between runs) ------------------------------
const META = {
  power:    { max: 6, baseCost: 60,  step: 55 },  // +5% damage
  vigor:    { max: 6, baseCost: 55,  step: 50 },  // +14 max HP
  speed:    { max: 5, baseCost: 70,  step: 60 },  // +4% move
  growth:   { max: 5, baseCost: 80,  step: 70 },  // +8% XP
  fortune:  { max: 5, baseCost: 75,  step: 70 },  // +12% gold
  recovery: { max: 4, baseCost: 110, step: 100 }, // +0.4 HP/s
  revive:   { max: 1, baseCost: 400, step: 0 },   // +1 revive
};
function metaCost(id, lvl) {
  const m = META[id];
  return m.baseCost + m.step * lvl;
}
function metaBonus(id, lvl) {
  switch (id) {
    case 'power': return lvl * 0.05;
    case 'vigor': return lvl * 14;
    case 'speed': return lvl * 0.04;
    case 'growth': return lvl * 0.08;
    case 'fortune': return lvl * 0.12;
    case 'recovery': return lvl * 0.4;
    case 'revive': return lvl;
    default: return 0;
  }
}

const RUN_GOAL_SECONDS = 15 * 60; // survive 15 minutes, then beat the Overlord
