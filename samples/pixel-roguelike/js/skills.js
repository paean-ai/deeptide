// ==================== 技能池 ====================
// 每种技能可无限升级，每次选择同名技能则等级+1
// effect 接受 (player, level) 并更新 player 的属性

const SKILL_VISUALS = {
  melee: '#f2c14e',
  ranged: '#a9e8ff',
  attackspeed: '#ff8a3d',
  crit: '#f3f7ff',
  area: '#b66cff',
  maxhp: '#43d17a',
  shield: '#2f80ed',
  dodge: '#83d8ff',
  hpregen: '#b5f47a',
  revive: '#f2c14e',
  speed: '#43d17a',
  lifesteal: '#e05243',
  bounce: '#1abc9c',
  freeze: '#a9e8ff',
  burn: '#ff6b35',
  minion: '#b66cff',
  thorns: '#43d17a',
  expplus: '#f2c14e',
  magnet: '#b66cff',
  deathnova: '#c52f35',
};

const SKILL_GLYPHS = {
  melee: [
    '..1....',
    '...1...',
    '...1...',
    '..111..',
    '.1.1...',
    '1..1...',
    '...1...',
  ],
  ranged: [
    '..111..',
    '.1...1.',
    '1.....1',
    '..1111.',
    '...1...',
    '..1....',
    '.1.....',
  ],
  shield: [
    '.11111.',
    '1.....1',
    '1.111.1',
    '1.111.1',
    '.1...1.',
    '..1.1..',
    '...1...',
  ],
  freeze: [
    '...1...',
    '1..1..1',
    '.1.1.1.',
    '..111..',
    '.1.1.1.',
    '1..1..1',
    '...1...',
  ],
  burn: [
    '...1...',
    '..111..',
    '.1111..',
    '..1111.',
    '.11111.',
    '.1111..',
    '..11...',
  ],
  minion: [
    '.11111.',
    '1.1.1.1',
    '1.....1',
    '1.111.1',
    '.1...1.',
    '..1.1..',
    '.1...1.',
  ],
  magnet: [
    '11...11',
    '11...11',
    '11...11',
    '1111111',
    '..111..',
    '...1...',
    '.......',
  ],
  deathnova: [
    '...1...',
    '.1.1.1.',
    '..111..',
    '1111111',
    '..111..',
    '.1.1.1.',
    '...1...',
  ],
  default: [
    '...1...',
    '..111..',
    '.11111.',
    '1111111',
    '.11111.',
    '..111..',
    '...1...',
  ],
};

const SKILLS_DATA = [
  // ======== 红色系 - 攻击 ========
  {
    id: 'melee',
    name: '近战强化',
    icon: '⚔️',
    color: '#e74c3c',
    desc: lv => `近战攻击力 +${lv * 5}`,
    effect: (p, lv) => { p._meleeBonus = lv * 5; },
  },
  {
    id: 'ranged',
    name: '远程强化',
    icon: '🏹',
    color: '#e74c3c',
    desc: lv => `远程攻击力 +${lv * 4}`,
    effect: (p, lv) => { p._rangedBonus = lv * 4; },
  },
  {
    id: 'attackspeed',
    name: '攻速提升',
    icon: '💨',
    color: '#e67e22',
    desc: lv => `攻速 +${(lv * 8).toFixed(0)}%`,
    effect: (p, lv) => { p._attackSpeedMul = Math.max(0.35, 1 - lv * 0.08); },
  },
  {
    id: 'crit',
    name: '暴击一击',
    icon: '💥',
    color: '#e74c3c',
    desc: lv => `暴击率 +${(lv * 4).toFixed(0)}%`,
    effect: (p, lv) => { p._critBonus = Math.min(0.65, lv * 0.04); },
  },
  {
    id: 'area',
    name: '范围扩大',
    icon: '🌊',
    color: '#e67e22',
    desc: lv => `攻击范围 +${lv * 8}%`,
    effect: (p, lv) => { p._areaMul = 1 + lv * 0.08; },
  },

  // ======== 蓝色系 - 防御 ========
  {
    id: 'maxhp',
    name: '生命强化',
    icon: '❤️',
    color: '#3498db',
    desc: lv => `最大生命 +${lv * 25}`,
    effect: (p, lv) => {
      const add = 25;
      p.maxHP += add;
      p.hp += add;
    },
  },
  {
    id: 'shield',
    name: '护盾',
    icon: '🛡️',
    color: '#3498db',
    desc: lv => `受伤 -${(lv * 3).toFixed(0)}%`,
    effect: (p, lv) => { p._shieldMul = Math.max(0.35, 1 - lv * 0.03); },
  },
  {
    id: 'dodge',
    name: '闪避',
    icon: '🌀',
    color: '#85c1e9',
    desc: lv => `闪避 +${(lv * 3).toFixed(0)}%`,
    effect: (p, lv) => { p._dodgeBonus = Math.min(0.6, lv * 0.03); },
  },
  {
    id: 'hpregen',
    name: '自动回血',
    icon: '💚',
    color: '#2ecc71',
    desc: lv => `每秒回血 +${lv * 2}`,
    effect: (p, lv) => { p._regenBonus = lv * 2; },
  },
  {
    id: 'revive',
    name: '重生',
    icon: '♻️',
    color: '#3498db',
    desc: lv => `重生 +${lv} 次 (满血复活)`,
    effect: (p, lv) => {
      p._reviveMax = lv;
      p._revives = Math.min(p._reviveMax, p._revives + 1);
    },
  },

  // ======== 绿色系 - 特殊 ========
  {
    id: 'speed',
    name: '移速提升',
    icon: '👟',
    color: '#2ecc71',
    desc: lv => `移动速度 +${(lv * 0.15).toFixed(2)}`,
    effect: (p, lv) => { p._speedBonus = lv * 0.15; },
  },
  {
    id: 'lifesteal',
    name: '生命偷取',
    icon: '🩸',
    color: '#e74c3c',
    desc: lv => `吸血 +${(lv * 2).toFixed(0)}%`,
    effect: (p, lv) => { p._lifeStealBonus = lv * 0.02; },
  },
  {
    id: 'bounce',
    name: '弹射攻击',
    icon: '🔄',
    color: '#1abc9c',
    desc: lv => `弹射 +${lv} 次`,
    effect: (p, lv) => { p._bounceCount = lv; },
  },
  {
    id: 'freeze',
    name: '冰冻光环',
    icon: '❄️',
    color: '#85c1e9',
    desc: lv => `减速周围敌人 ${(lv * 8).toFixed(0)}%`,
    effect: (p, lv) => { p._freezePower = lv * 0.08; },
  },
  {
    id: 'burn',
    name: '灼烧光环',
    icon: '🔥',
    color: '#e67e22',
    desc: lv => `每秒灼烧 ${lv * 3} 伤害`,
    effect: (p, lv) => { p._burnPower = lv * 3; },
  },

  // ======== 紫色系 - 召唤/特殊 ========
  {
    id: 'minion',
    name: '小跟班',
    icon: '👻',
    color: '#9b59b6',
    desc: lv => `召唤 ${lv} 个跟班攻击`,
    effect: (p, lv) => { p._minionCount = lv; },
  },
  {
    id: 'thorns',
    name: '尖刺反弹',
    icon: '🌵',
    color: '#27ae60',
    desc: lv => `反弹 ${(lv * 5).toFixed(0)}% 伤害`,
    effect: (p, lv) => { p._thorns = lv * 0.05; },
  },
  {
    id: 'expplus',
    name: '经验加成',
    icon: '⭐',
    color: '#f1c40f',
    desc: lv => `经验 +${(lv * 15).toFixed(0)}%`,
    effect: (p, lv) => { p._expMul = 1 + lv * 0.15; },
  },
  {
    id: 'magnet',
    name: '磁铁',
    icon: '🧲',
    color: '#f1c40f',
    desc: lv => `拾取范围 +${lv * 30}`,
    effect: (p, lv) => { p._magnetBonus = lv * 30; },
  },
  {
    id: 'deathnova',
    name: '死亡爆炸',
    icon: '💣',
    color: '#e74c3c',
    desc: lv => `击杀爆炸伤害 ${lv * 15}`,
    effect: (p, lv) => { p._novaDamage = lv * 15; },
  },
];

// ==================== 技能系统 ====================
class SkillSystem {
  constructor(player) {
    this.player = player;
    this.owned = {}; // skillId -> level
    this.allSkills = SKILLS_DATA;
  }

  getSkill(id) {
    return this.allSkills.find(s => s.id === id);
  }

  getLevel(id) {
    return this.owned[id] || 0;
  }

  addSkill(id) {
    if (this.owned[id]) {
      this.owned[id]++;
    } else {
      this.owned[id] = 1;
    }
    const skill = this.getSkill(id);
    if (skill) {
      skill.effect(this.player, this.owned[id]);
    }
  }

  // 获取 3 个随机技能（不重复）
  getRandomChoices(count = 3) {
    const pool = [...this.allSkills];
    const choices = [];
    while (choices.length < count && pool.length > 0) {
      const idx = Math.floor(Math.random() * pool.length);
      const skill = pool.splice(idx, 1)[0];
      choices.push(skill);
    }
    return choices;
  }
}
