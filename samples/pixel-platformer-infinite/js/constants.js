const VIEW_W = 960;
const VIEW_H = 540;
const GRAVITY = 0.62;
const FRICTION = 0.82;
const TILE = 24;

const COLORS = {
  sky0: '#08101d',
  sky1: '#10192b',
  metal: '#273241',
  metalTop: '#5b6a7d',
  cyan: '#61e5ff',
  green: '#52dc88',
  gold: '#f5c451',
  red: '#ec5b56',
  violet: '#ad7dff',
  ink: '#05070b',
  white: '#edf4ff',
};

function xpForLevel(level) {
  return Math.floor(70 + level * 24 + Math.pow(level, 1.45) * 9);
}

function mulberry32(seed) {
  return function rand() {
    let t = seed += 0x6D2B79F5;
    t = Math.imul(t ^ t >>> 15, t | 1);
    t ^= t + Math.imul(t ^ t >>> 7, t | 61);
    return ((t ^ t >>> 14) >>> 0) / 4294967296;
  };
}

const SKILLS = [
  { id: 'blade', name: { en: 'Plasma Blade', zh: '等离子刃' }, color: COLORS.red, desc: { en: lv => `Attack damage +${lv * 4}`, zh: lv => `攻击伤害 +${lv * 4}` }, apply: (p, lv) => { p.attack = 16 + lv * 4; } },
  { id: 'heart', name: { en: 'Alloy Heart', zh: '合金心脏' }, color: COLORS.green, desc: { en: lv => `Max HP +${lv * 18}`, zh: lv => `最大生命 +${lv * 18}` }, apply: (p, lv) => { p.maxHp = 100 + lv * 18; p.hp = Math.min(p.maxHp, p.hp + 18); } },
  { id: 'jump', name: { en: 'Anti-Gravity Boots', zh: '反重力靴' }, color: COLORS.cyan, desc: { en: lv => `Jump count +${Math.floor(lv / 3) + 1}, stronger lift`, zh: lv => `跳跃次数 +${Math.floor(lv / 3) + 1}，跳力提升` }, apply: (p, lv) => { p.maxJumps = 1 + Math.floor(lv / 3); p.jumpPower = 12.4 + lv * 0.16; } },
  { id: 'dash', name: { en: 'Phase Dash', zh: '相位冲刺' }, color: COLORS.violet, desc: { en: lv => `Dash cooldown -${Math.min(75, lv * 5)}%`, zh: lv => `冲刺冷却 -${Math.min(75, lv * 5)}%` }, apply: (p, lv) => { p.dashCooldownMax = Math.max(14, 70 - lv * 3.5); } },
  { id: 'crit', name: { en: 'Weakpoint Scan', zh: '弱点解析' }, color: COLORS.gold, desc: { en: lv => `Crit chance +${Math.min(80, lv * 4)}%`, zh: lv => `暴击率 +${Math.min(80, lv * 4)}%` }, apply: (p, lv) => { p.crit = Math.min(0.8, 0.04 + lv * 0.04); } },
  { id: 'guard', name: { en: 'Energy Guard', zh: '能量护盾' }, color: '#7fa7ff', desc: { en: lv => `Damage reduction +${Math.min(70, lv * 4)}%`, zh: lv => `受伤减免 +${Math.min(70, lv * 4)}%` }, apply: (p, lv) => { p.guard = Math.min(0.7, lv * 0.04); } },
  { id: 'magnet', name: { en: 'Chip Magnet', zh: '晶片磁吸' }, color: '#ff80c7', desc: { en: lv => `Pickup range +${lv * 24}`, zh: lv => `拾取范围 +${lv * 24}` }, apply: (p, lv) => { p.magnet = 70 + lv * 24; } },
  { id: 'tempo', name: { en: 'Combo Tempo', zh: '连击节拍' }, color: '#ff9d52', desc: { en: lv => `Attack interval -${Math.min(65, lv * 4)}%`, zh: lv => `攻击间隔 -${Math.min(65, lv * 4)}%` }, apply: (p, lv) => { p.attackCooldownMax = Math.max(10, 28 - lv * 1.25); } },
  { id: 'coin', name: { en: 'Bounty Protocol', zh: '赏金协议' }, color: '#f5e16f', desc: { en: lv => `XP and chips +${lv * 10}%`, zh: lv => `经验和晶片 +${lv * 10}%` }, apply: (p, lv) => { p.rewardMul = 1 + lv * 0.1; } },
];
