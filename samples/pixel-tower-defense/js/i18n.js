// Pixel Tower Defense - localization
const LANG_KEY = 'pixel-td-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'PIXEL TOWER DEFENSE',
    subtitle: 'Build, upgrade, hold the line.',
    play: 'PLAY',
    howto: 'Place towers on grass tiles. Tap a tower to upgrade or sell. Survive every wave.',
    selectLevel: 'SELECT LEVEL',
    back: 'Back',
    locked: 'LOCKED',
    lives: 'Lives',
    gold: 'Gold',
    wave: 'Wave',
    startWave: 'Start Wave',
    callNext: 'Call Next (+%G)',
    nextIn: 'Next wave',
    speed: 'Speed',
    pause: 'Pause',
    paused: 'PAUSED',
    resume: 'Resume',
    quit: 'Quit',
    restart: 'Restart',
    sell: 'Sell',
    upgrade: 'Upgrade',
    target: 'Target',
    targetFirst: 'First',
    targetLast: 'Last',
    targetStrong: 'Strong',
    targetClose: 'Close',
    victory: 'VICTORY',
    defeat: 'DEFEAT',
    levelClear: 'Level Cleared!',
    coreLost: 'Your core was overrun.',
    nextLevel: 'Next Level',
    retry: 'Retry',
    menu: 'Menu',
    endless: 'Endless',
    waveOf: (a, b) => `Wave ${a} / ${b}`,
    waveEndless: a => `Wave ${a} (Endless)`,
    cantAfford: 'Not enough gold',
    blocked: 'Blocked tile',
    maxTier: 'Max tier reached',
    dmg: 'DMG',
    rng: 'RNG',
    spd: 'SPD',
    sells: amt => `Sell +${amt}`,
    tutorial: 'Tap a tower below, then tap a grass tile to build it.',
  },
  zh: {
    title: '像素塔防',
    subtitle: '建造、升级、守住防线。',
    play: '开始游戏',
    howto: '在草地格子上建造防御塔。点击塔可升级或出售。守住每一波进攻。',
    selectLevel: '选择关卡',
    back: '返回',
    locked: '未解锁',
    lives: '生命',
    gold: '金币',
    wave: '波次',
    startWave: '开始进攻',
    callNext: '提前召唤 (+%G)',
    nextIn: '下一波',
    speed: '速度',
    pause: '暂停',
    paused: '已暂停',
    resume: '继续',
    quit: '退出',
    restart: '重新开始',
    sell: '出售',
    upgrade: '升级',
    target: '目标',
    targetFirst: '最前',
    targetLast: '最后',
    targetStrong: '最强',
    targetClose: '最近',
    victory: '胜利',
    defeat: '失败',
    levelClear: '关卡通关！',
    coreLost: '核心已被攻陷。',
    nextLevel: '下一关',
    retry: '重试',
    menu: '主菜单',
    endless: '无尽',
    waveOf: (a, b) => `第 ${a} / ${b} 波`,
    waveEndless: a => `第 ${a} 波（无尽）`,
    cantAfford: '金币不足',
    blocked: '无法建造',
    maxTier: '已达最高等级',
    dmg: '伤害',
    rng: '射程',
    spd: '攻速',
    sells: amt => `出售 +${amt}`,
    tutorial: '先点击下方的塔，再点击草地格子建造。',
  },
};

// Tower / enemy / branch display names keyed by id: [en, zh]
const NAMES = {
  arrow: ['Archer', '弓箭手'],
  cannon: ['Cannon', '加农炮'],
  frost: ['Frost', '寒冰塔'],
  arcane: ['Arcane', '奥术塔'],
  sniper: ['Sniper', '狙击手'],
  rapid: ['Repeater', '速射手'],
  mortar: ['Mortar', '迫击炮'],
  demolisher: ['Demolisher', '爆破炮'],
  glacier: ['Glacier', '冰川塔'],
  blizzard: ['Blizzard', '暴雪塔'],
  storm: ['Storm', '风暴塔'],
  tesla: ['Tesla', '特斯拉'],
  grunt: ['Grunt', '步兵'],
  runner: ['Runner', '疾行者'],
  swarm: ['Swarmling', '虫群'],
  armored: ['Juggernaut', '重甲兵'],
  flyer: ['Wraith', '幽翼'],
  healer: ['Mender', '医疗兵'],
  boss: ['Warlord', '首领'],
};

const TOWER_DESC = {
  arrow: ['Fast single-target physical shots.', '快速单体物理攻击。'],
  cannon: ['Slow splash damage. Cannot hit air.', '缓慢的范围伤害，无法攻击空中。'],
  frost: ['Low damage, slows enemies. Magic.', '低伤害但能减速敌人，魔法属性。'],
  arcane: ['Chain lightning, hits air. Magic.', '连锁闪电，可攻击空中，魔法属性。'],
};

function t(key, ...args) {
  const v = TEXT[currentLang][key] ?? TEXT.en[key] ?? key;
  return typeof v === 'function' ? v(...args) : v;
}
function name(id) {
  const e = NAMES[id];
  return e ? (currentLang === 'zh' ? e[1] : e[0]) : id;
}
function towerDesc(id) {
  const e = TOWER_DESC[id];
  return e ? (currentLang === 'zh' ? e[1] : e[0]) : '';
}
function applyStaticText() {
  document.documentElement.lang = currentLang === 'zh' ? 'zh-CN' : 'en';
  document.querySelectorAll('[data-i18n]').forEach(el => {
    el.textContent = t(el.dataset.i18n);
  });
}
function setupLanguageToggle(onChange) {
  const btn = document.getElementById('lang-toggle');
  if (btn) {
    btn.onclick = () => {
      currentLang = currentLang === 'en' ? 'zh' : 'en';
      localStorage.setItem(LANG_KEY, currentLang);
      btn.textContent = currentLang === 'en' ? '中文' : 'EN';
      applyStaticText();
      if (onChange) onChange();
    };
    btn.textContent = currentLang === 'en' ? '中文' : 'EN';
  }
  applyStaticText();
}
