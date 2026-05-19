// Pixel Street Brawl - content data: dimensions, fighter stats, wave tuning.

const VW = 480, VH = 300;
const GROUND = VH - 54;        // y of the street floor

const PLAYER = {
  maxHp: 110, speed: 158, jump: 392, gravity: 1180, r: 16,
  punch: { cd: 0.30, dmg: 9,  range: 44, knock: 90,  dur: 0.20 },
  kick:  { cd: 0.54, dmg: 19, range: 56, knock: 215, dur: 0.32 },
  comboWindow: 0.55,           // seconds to chain the next punch
};

// Enemy archetypes. `windup` = telegraph before the hit lands.
const ENEMIES = {
  thug:  { hp: 30,  speed: 64, dmg: 8,  range: 40, atkCd: 1.3, windup: 0.32, r: 15, score: 100, color: '#c0563f' },
  brute: { hp: 78,  speed: 42, dmg: 16, range: 46, atkCd: 1.9, windup: 0.46, r: 21, score: 260, color: '#7a5fb0' },
  boss:  { hp: 420, speed: 50, dmg: 22, range: 58, atkCd: 1.5, windup: 0.4,  r: 27, score: 2200, color: '#c0473f', boss: true },
};

const BOSS_EVERY = 5;
const HEAL_DROP_CHANCE = 0.16;
const HEAL_AMOUNT = 24;

// Enemy list for a normal wave.
function waveEnemies(wave) {
  const list = [];
  const n = 2 + Math.floor(wave * 1.1);
  for (let i = 0; i < n; i++) {
    list.push(wave >= 3 && Math.random() < 0.25 + wave * 0.02 ? 'brute' : 'thug');
  }
  return list;
}
function bossWaveEnemies(wave) {
  return ['boss', 'thug', 'thug'];
}
function enemyScale(wave) { return 1 + (wave - 1) * 0.13; }
