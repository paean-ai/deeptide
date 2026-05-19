// Pixel Sky Raiders - content data: dims, enemies, weapons, powerups, waves

const VW = 420;          // canvas logical width
const VH = 640;          // canvas logical height

// Player weapon tiers (1..5). Each lists muzzle offsets + bullet angles.
const WEAPONS = [
  null,
  { cd: 0.22, shots: [{ dx: 0, a: 0 }] },
  { cd: 0.20, shots: [{ dx: -7, a: 0 }, { dx: 7, a: 0 }] },
  { cd: 0.19, shots: [{ dx: 0, a: 0 }, { dx: -9, a: -0.20 }, { dx: 9, a: 0.20 }] },
  { cd: 0.15, shots: [{ dx: -6, a: 0 }, { dx: 6, a: 0 }, { dx: -11, a: -0.22 }, { dx: 11, a: 0.22 }] },
  { cd: 0.13, shots: [{ dx: 0, a: 0 }, { dx: -7, a: -0.16 }, { dx: 7, a: 0.16 }, { dx: -13, a: -0.34 }, { dx: 13, a: 0.34 }] },
];
const MAX_WEAPON = 5;

// Enemy archetypes. hp/score scale with wave at spawn time.
const ENEMIES = {
  drone:  { hp: 8,  speed: 95,  score: 100, r: 13, fire: 0,    move: 'dive',  color: '#ff6b6b' },
  weaver: { hp: 13, speed: 78,  score: 150, r: 13, fire: 0,    move: 'sine',  color: '#ffd24d' },
  turret: { hp: 22, speed: 52,  score: 260, r: 15, fire: 1.5,  move: 'hover', color: '#7ad0ff' },
  tank:   { hp: 52, speed: 38,  score: 520, r: 20, fire: 2.1,  move: 'dive',  color: '#b78cff' },
};

const POWERUPS = {
  power:  { color: '#ffd24d', glyph: 'P' },
  bomb:   { color: '#ff8a3c', glyph: 'B' },
  shield: { color: '#7ad0ff', glyph: 'S' },
};

const PLAYER = {
  maxHp: 100, speed: 250, r: 12,
  startBombs: 2, shieldTime: 6, invulnTime: 1.2,
};

const BOSS_EVERY = 5;    // a boss appears on every 5th wave

function depthScale(wave) { return 1 + (wave - 1) * 0.14; }

// Builds the spawn list for a normal wave.
function waveSpawns(wave) {
  const list = [];
  const n = 5 + Math.floor(wave * 1.6);
  const pool = ['drone'];
  if (wave >= 2) pool.push('weaver');
  if (wave >= 4) pool.push('turret');
  if (wave >= 6) pool.push('tank');
  for (let i = 0; i < n; i++) {
    list.push(pool[Math.floor(Math.random() * pool.length)]);
  }
  return list;
}

function bossStats(wave) {
  const s = depthScale(wave);
  return { hp: Math.round(420 * s), score: 5000 + wave * 400, r: 46 };
}
