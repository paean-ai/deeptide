// ==================== Canvas ====================
const CANVAS_W = 900;
const CANVAS_H = 700;
const WORLD_W = 3000;
const WORLD_H = 3000;

// ==================== 像素调色板 ====================
const PALETTE = {
  transparent: null,
  black:  '#0a0a0a',
  dark:   '#1a1a2e',
  red:    '#e74c3c',
  darkRed:'#c0392b',
  orange: '#e67e22',
  yellow: '#f1c40f',
  green:  '#2ecc71',
  darkGreen:'#27ae60',
  teal:   '#1abc9c',
  blue:   '#3498db',
  lightBlue:'#85c1e9',
  purple: '#9b59b6',
  pink:   '#ff6b81',
  white:  '#ecf0f1',
  silver: '#bdc3c7',
  gray:   '#7f8c8d',
  brown:  '#8B4513',
  skin:   '#f5cba7',
};

// ==================== 玩家初始属性 ====================
const PLAYER_INIT = {
  maxHP:       100,
  hp:          100,
  speed:       2.8,
  attack:      10,
  rangedAttack:8,
  attackRange: 50,      // 近战范围
  attackSpeed: 400,     // 毫秒间隔
  rangedSpeed: 600,
  projectileSpeed: 5,
  critChance:  0.05,
  critDamage:  1.5,
  dodge:       0,
  lifeSteal:   0,
  areaRange:   30,
  hpRegen:     0,
  size:        14,
};

// ==================== 经验 ====================
function expForLevel(lv) {
  return Math.floor(80 + lv * 35 + Math.pow(lv, 1.5) * 2);
}

// ==================== 敌人 ====================
const ENEMY_TYPES = {
  slime: {
    name: '史莱姆',
    color: '#2ecc71',
    hp: 20, attack: 5, speed: 1.2, size: 10,
    exp: 8, score: 1,
  },
  bat: {
    name: '蝙蝠',
    color: '#9b59b6',
    hp: 14, attack: 4, speed: 2.2, size: 8,
    exp: 6, score: 1,
  },
  skeleton: {
    name: '骷髅',
    color: '#bdc3c7',
    hp: 35, attack: 8, speed: 1.5, size: 12,
    exp: 12, score: 2,
  },
  orc: {
    name: '兽人',
    color: '#8B4513',
    hp: 60, attack: 14, speed: 1.0, size: 14,
    exp: 20, score: 3,
  },
  elite: {
    name: '精英',
    color: '#e67e22',
    hp: 120, attack: 22, speed: 1.3, size: 16,
    exp: 40, score: 5,
  },
  goblin: {
    name: '哥布林',
    color: '#27ae60',
    hp: 30, attack: 7, speed: 1.8, size: 11,
    exp: 10, score: 2,
  },
  ghost: {
    name: '幽灵',
    color: '#85c1e9',
    hp: 50, attack: 18, speed: 1.6, size: 13,
    exp: 18, score: 3,
  },
  demon: {
    name: '恶魔',
    color: '#c0392b',
    hp: 100, attack: 26, speed: 1.1, size: 16,
    exp: 35, score: 5,
  },
  spider: {
    name: '毒蛛',
    color: '#7d3c98',
    hp: 24, attack: 6, speed: 2.0, size: 9,
    exp: 11, score: 2,
  },
  golem: {
    name: '石巨人',
    color: '#5d6d7e',
    hp: 170, attack: 28, speed: 0.7, size: 18,
    exp: 55, score: 8,
  },
  imp: {
    name: '小恶魔',
    color: '#ff8a3d',
    hp: 45, attack: 16, speed: 1.9, size: 10,
    exp: 16, score: 3,
  },
  wraith: {
    name: '怨灵',
    color: '#a9e8ff',
    hp: 80, attack: 24, speed: 1.4, size: 14,
    exp: 30, score: 4,
  },
  cultist: {
    name: '邪教徒',
    color: '#8a5fd0',
    hp: 55, attack: 20, speed: 1.5, size: 12,
    exp: 22, score: 4,
  },
  behemoth: {
    name: '巨兽',
    color: '#7a5a3a',
    hp: 240, attack: 34, speed: 0.65, size: 20,
    exp: 80, score: 12,
  },
  boss: {
    name: 'Boss',
    color: '#e74c3c',
    hp: 400, attack: 40, speed: 0.8, size: 22,
    exp: 150, score: 20,
  },
};

// ==================== 波次 ====================
function waveConfig(wave) {
  const count = Math.min(5 + wave * 2, 35);
  let canSpawn = ['slime', 'bat'];
  if (wave >= 2) canSpawn.push('skeleton');
  if (wave >= 3) canSpawn.push('goblin');
  if (wave >= 4) canSpawn.push('spider');
  if (wave >= 4) canSpawn.push('orc');
  if (wave >= 6) canSpawn.push('ghost');
  if (wave >= 8) canSpawn.push('elite');
  if (wave >= 9) canSpawn.push('golem');
  if (wave >= 10) canSpawn.push('demon');
  if (wave >= 5) canSpawn.push('imp');
  if (wave >= 7) canSpawn.push('wraith');
  if (wave >= 6) canSpawn.push('cultist');
  if (wave >= 11) canSpawn.push('behemoth');
  if (wave % 5 === 0) {
    const bossHp = 300 + wave * 80;
    return { count: count + 3, types: canSpawn, hasBoss: true, bossHp };
  }
  return { count, types: canSpawn, hasBoss: false };
}

// ==================== 颜色工具 ====================
function hexToRgba(hex, a) {
  const r = parseInt(hex.slice(1,3),16);
  const g = parseInt(hex.slice(3,5),16);
  const b = parseInt(hex.slice(5,7),16);
  return `rgba(${r},${g},${b},${a})`;
}
