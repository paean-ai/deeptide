const LANG_KEY = 'pixel-roguelike-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    wave: 'Wave',
    skills: 'Skills',
    levelUp: 'Level Up!',
    chooseSkill: 'Choose one skill upgrade',
    gameOver: 'Game Over',
    restart: 'Restart',
    pause: 'Pause',
    paused: 'Paused',
    resume: 'Resume',
    kills: 'Kills',
    langToggle: '中文',
    waveText: n => `Wave ${n}`,
    gameoverInfo: (level, wave, kills) => `Lv.${level} | Wave ${wave} | Kills ${kills}`,
    runEarned: amount => `Run essence +${amount}`,
    accountInfo: (essence, bestWave) => `Account essence ${essence} | Best wave ${bestWave}`,
    buyUpgrade: cost => `Upgrade (${cost})`,
    maxed: 'MAX',
  },
  zh: {
    wave: '波次',
    skills: '技能',
    levelUp: '升级!',
    chooseSkill: '选择一个技能强化',
    gameOver: '游戏结束',
    restart: '再来一局',
    pause: '暂停',
    paused: '已暂停',
    resume: '继续',
    kills: '击杀',
    langToggle: 'English',
    waveText: n => `第 ${n} 波`,
    gameoverInfo: (level, wave, kills) => `Lv.${level} | 波次 ${wave} | 击杀 ${kills}`,
    runEarned: amount => `本局精华 +${amount}`,
    accountInfo: (essence, bestWave) => `账户精华 ${essence} | 最高波次 ${bestWave}`,
    buyUpgrade: cost => `升级 (${cost})`,
    maxed: '已满',
  },
};

const SKILL_TEXT = {
  melee: ['Melee Boost', '近战强化', lv => `Melee damage +${lv * 5}`, lv => `近战攻击力 +${lv * 5}`],
  ranged: ['Ranged Boost', '远程强化', lv => `Ranged damage +${lv * 4}`, lv => `远程攻击力 +${lv * 4}`],
  attackspeed: ['Attack Speed', '攻速提升', lv => `Attack speed +${(lv * 8).toFixed(0)}%`, lv => `攻速 +${(lv * 8).toFixed(0)}%`],
  crit: ['Critical Strike', '暴击一击', lv => `Crit chance +${(lv * 4).toFixed(0)}%`, lv => `暴击率 +${(lv * 4).toFixed(0)}%`],
  area: ['Wider Arc', '范围扩大', lv => `Attack area +${lv * 8}%`, lv => `攻击范围 +${lv * 8}%`],
  maxhp: ['Vitality', '生命强化', lv => `Max HP +${lv * 25}`, lv => `最大生命 +${lv * 25}`],
  shield: ['Shielding', '护盾', lv => `Damage taken -${(lv * 3).toFixed(0)}%`, lv => `受伤 -${(lv * 3).toFixed(0)}%`],
  dodge: ['Evasion', '闪避', lv => `Dodge +${(lv * 3).toFixed(0)}%`, lv => `闪避 +${(lv * 3).toFixed(0)}%`],
  hpregen: ['Regeneration', '自动回血', lv => `HP regen +${lv * 2}/s`, lv => `每秒回血 +${lv * 2}`],
  revive: ['Revive', '重生', lv => `Revive ${lv} time(s) at full HP`, lv => `重生 +${lv} 次 (满血复活)`],
  speed: ['Move Speed', '移速提升', lv => `Move speed +${(lv * 0.15).toFixed(2)}`, lv => `移动速度 +${(lv * 0.15).toFixed(2)}`],
  lifesteal: ['Life Steal', '生命偷取', lv => `Life steal +${(lv * 2).toFixed(0)}%`, lv => `吸血 +${(lv * 2).toFixed(0)}%`],
  bounce: ['Ricochet', '弹射攻击', lv => `Projectile bounces +${lv}`, lv => `弹射 +${lv} 次`],
  freeze: ['Frost Aura', '冰冻光环', lv => `Slow nearby enemies ${(lv * 8).toFixed(0)}%`, lv => `减速周围敌人 ${(lv * 8).toFixed(0)}%`],
  burn: ['Burn Aura', '灼烧光环', lv => `${lv * 3} burn damage per second`, lv => `每秒灼烧 ${lv * 3} 伤害`],
  minion: ['Wisp Ally', '小跟班', lv => `Summon ${lv} ally minion(s)`, lv => `召唤 ${lv} 个跟班攻击`],
  thorns: ['Thorns', '尖刺反弹', lv => `Reflect ${(lv * 5).toFixed(0)}% damage`, lv => `反弹 ${(lv * 5).toFixed(0)}% 伤害`],
  expplus: ['XP Boost', '经验加成', lv => `XP +${(lv * 15).toFixed(0)}%`, lv => `经验 +${(lv * 15).toFixed(0)}%`],
  magnet: ['Magnet', '磁铁', lv => `Pickup range +${lv * 30}`, lv => `拾取范围 +${lv * 30}`],
  deathnova: ['Death Nova', '死亡爆炸', lv => `Kill explosion damage ${lv * 15}`, lv => `击杀爆炸伤害 ${lv * 15}`],
};

function t(key, ...args) {
  const value = TEXT[currentLang][key] ?? TEXT.en[key] ?? key;
  return typeof value === 'function' ? value(...args) : value;
}

function skillName(id) {
  const entry = SKILL_TEXT[id];
  if (!entry) return id;
  return currentLang === 'zh' ? entry[1] : entry[0];
}

function skillDesc(id, level) {
  const entry = SKILL_TEXT[id];
  if (!entry) return '';
  return (currentLang === 'zh' ? entry[3] : entry[2])(level);
}

function applyStaticText() {
  document.documentElement.lang = currentLang === 'zh' ? 'zh-CN' : 'en';
  document.querySelectorAll('[data-i18n]').forEach(el => {
    el.textContent = t(el.dataset.i18n);
  });
}

function setupLanguageToggle(onChange) {
  const btn = document.getElementById('lang-toggle');
  if (!btn) return;
  btn.onclick = () => {
    currentLang = currentLang === 'en' ? 'zh' : 'en';
    localStorage.setItem(LANG_KEY, currentLang);
    applyStaticText();
    if (onChange) onChange();
  };
  applyStaticText();
}
