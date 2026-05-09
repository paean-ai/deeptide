const LANG_KEY = 'void-descent-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    subtitle: '— infinite rogue-like dungeon crawler —',
    lore: 'The <span>Void</span> stretches endlessly beneath reality.<br>Each floor warps space and time, spawning ever-stronger horrors.<br>Collect <span>Void Essence</span>, choose <span>upgrades</span>,<br>and descend as deep as you dare.<br><br><span>No end. No mercy. Only the descent.</span>',
    begin: '▼ BEGIN DESCENT',
    controlsLanding: 'Arrow keys / WASD / vi keys to move<br><span style="color:var(--accent2)">></span> to descend stairs &nbsp;|&nbsp;<span style="color:var(--accent2)">.</span> to wait a turn<br>Touch drag on mobile',
    continueConfirm: floor => `Continue from Floor ${floor}?\n\nOK = Continue\nCancel = New Game`,
    floor: 'Floor',
    hp: 'HP',
    atk: 'ATK',
    def: 'DEF',
    hint: 'move: arrows/wasd/vi | descend stairs: walk on / Enter / > | wait: .',
    firstMessage: 'Use arrow keys to move. Find > to descend.',
    enterFloor: (floor, theme) => `Floor ${floor} [${theme}] — Find the stairs › and walk onto them`,
    youDied: 'YOU DIED',
    floorReached: 'Floor reached',
    floorsCleared: 'Floors cleared',
    totalKills: 'Total kills',
    upgradesCollected: 'Upgrades collected',
    descendAgain: 'DESCEND AGAIN',
    returnTitle: 'Return to Title',
    cleared: '▼ FLOOR CLEARED ▼',
    skip: 'Skip upgrade & continue',
    floorMove: (a, b) => `Floor ${a} → ${b}`,
    langToggle: '中文',
  },
  zh: {
    subtitle: '— 无限 roguelike 地牢探索 —',
    lore: '<span>虚空</span>在现实之下无尽延伸。<br>每一层都会扭曲空间与时间，生成更强的怪物。<br>收集<span>虚空精华</span>，选择<span>升级</span>，<br>尽可能深入地下。<br><br><span>没有终点。没有仁慈。只有下潜。</span>',
    begin: '▼ 开始下潜',
    controlsLanding: '方向键 / WASD / vi 键移动<br><span style="color:var(--accent2)">></span> 下楼 &nbsp;|&nbsp;<span style="color:var(--accent2)">.</span> 等待一回合<br>移动端拖动操作',
    continueConfirm: floor => `从第 ${floor} 层继续？\n\nOK = 继续\nCancel = 新游戏`,
    floor: '层数',
    hp: '生命',
    atk: '攻击',
    def: '防御',
    hint: '移动：方向键/WASD/vi | 下楼：走到楼梯 / Enter / > | 等待：.',
    firstMessage: '使用方向键移动，找到 > 进入下一层。',
    enterFloor: (floor, theme) => `第 ${floor} 层 [${theme}] — 找到楼梯 › 并走上去`,
    youDied: '你死了',
    floorReached: '抵达层数',
    floorsCleared: '已清理层数',
    totalKills: '总击杀',
    upgradesCollected: '已获得升级',
    descendAgain: '再次下潜',
    returnTitle: '返回标题',
    cleared: '▼ 本层清理完成 ▼',
    skip: '跳过升级并继续',
    floorMove: (a, b) => `第 ${a} 层 → 第 ${b} 层`,
    langToggle: 'English',
  },
};

const UPGRADE_ZH = {
  vitality: ['生命力', '+25 最大生命'],
  power: ['力量', '+5 攻击'],
  armor: ['坚韧', '+3 防御'],
  crit: ['精准', '+8% 暴击率'],
  lifesteal: ['吸血', '+12% 吸血'],
  thorns: ['荆棘', '反弹 25% 伤害'],
  regen: ['再生', '每 5 回合回复 4 HP'],
  swift: ['迅捷', '+12% 免费移动概率'],
  doublestrike: ['双重打击', '+10% 二次攻击'],
  dodge: ['闪避', '+8% 闪避率'],
  berserk: ['狂战士', '生命低于 30% 时攻击 +40%'],
  shield: ['虚空护盾', '每层 +30 护盾'],
  scout: ['侦察', '+2 视野范围'],
  leech: ['灵魂汲取', '本层每击杀 +2 攻击'],
  assassin: ['刺客', '每层首次攻击必定暴击'],
  phoenix: ['凤凰', '每层以 50% HP 复活一次'],
  cleave: ['顺劈', '攻击命中相邻敌人'],
  alchemist: ['炼金术士', '药水治疗 +60%'],
  poison: ['毒刃', '攻击附毒 3 回合'],
  fortune: ['幸运', '敌人有 10% 未命中概率'],
  glasscanon: ['玻璃大炮', '+12 攻击，-8 最大生命'],
  juggernaut: ['重装战士', '+15 最大生命，-2 攻击'],
};

function t(key, ...args) {
  const value = TEXT[currentLang][key] ?? TEXT.en[key] ?? key;
  return typeof value === 'function' ? value(...args) : value;
}

function upgradeName(upgrade) {
  return currentLang === 'zh' && UPGRADE_ZH[upgrade.id] ? UPGRADE_ZH[upgrade.id][0] : upgrade.name;
}

function upgradeDesc(upgrade) {
  return currentLang === 'zh' && UPGRADE_ZH[upgrade.id] ? UPGRADE_ZH[upgrade.id][1] : upgrade.desc;
}

function applyStaticText() {
  document.documentElement.lang = currentLang === 'zh' ? 'zh-CN' : 'en';
  document.querySelectorAll('[data-i18n]').forEach(el => {
    el.textContent = t(el.dataset.i18n);
  });
  document.querySelectorAll('[data-i18n-html]').forEach(el => {
    el.innerHTML = t(el.dataset.i18nHtml);
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
