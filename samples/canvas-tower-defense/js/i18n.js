// Canvas Tower Defense - localization (English / 中文)
const LANG_KEY = 'canvas-tower-defense-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'TOWER DEFENSE',
    gold: 'Gold', lives: 'Lives', wave: 'Wave', score: 'Score', best: 'Best',
    towerLabel: 'Tower', startWave: 'Start Wave', upgrade: 'Upgrade', sell: 'Sell',
    rift: 'RIFT', core: 'CORE',
    tName_arrow: 'Arrow', tName_cannon: 'Cannon', tName_frost: 'Frost',
    tDesc_arrow: 'Fast single-target tower',
    tDesc_cannon: 'Slow splash damage',
    tDesc_frost: 'Slows enemies in range',
    introMsg: 'Build towers, then start the wave.',
    waveIncoming: (w, n) => `Wave ${w} incoming: ${n} enemies.`,
    needGoldFor: (c, name) => `Need ${c} gold for ${name}.`,
    towerBuilt: name => `${name} tower built.`,
    needGoldUpgrade: c => `Need ${c} gold to upgrade.`,
    towerUpgraded: (name, lv) => `${name} upgraded to Lv.${lv}.`,
    towerSold: 'Tower sold.',
    waveCleared: g => `Wave cleared. Bonus ${g} gold.`,
    statLine: (d, r) => `Damage ${d}  Range ${r}`,
    actionLine: (u, s) => `U: upgrade ${u}g   S: sell ${s}g`,
    towerHeading: name => `${name} Tower`,
    buildHint: 'Tap a pad to build. Keys 1/2/3 switch.',
    remaining: n => `${n} remaining`,
    enemiesNext: n => `${n} enemies next`,
    coreDestroyed: 'CORE DESTROYED',
    scoreLine: s => `Score ${s}`,
  },
  zh: {
    title: '塔防',
    gold: '金币', lives: '生命', wave: '波次', score: '分数', best: '最高',
    towerLabel: '防御塔', startWave: '开始波次', upgrade: '升级', sell: '出售',
    rift: '裂隙', core: '核心',
    tName_arrow: '箭塔', tName_cannon: '炮塔', tName_frost: '冰霜塔',
    tDesc_arrow: '快速单体攻击塔',
    tDesc_cannon: '缓慢的溅射伤害',
    tDesc_frost: '减速范围内的敌人',
    introMsg: '建造防御塔,然后开始波次。',
    waveIncoming: (w, n) => `第 ${w} 波来袭：${n} 个敌人。`,
    needGoldFor: (c, name) => `建造${name}需要 ${c} 金币。`,
    towerBuilt: name => `${name}建造完成。`,
    needGoldUpgrade: c => `升级需要 ${c} 金币。`,
    towerUpgraded: (name, lv) => `${name}升级到 ${lv} 级。`,
    towerSold: '已出售防御塔。',
    waveCleared: g => `波次清空,奖励 ${g} 金币。`,
    statLine: (d, r) => `伤害 ${d}  射程 ${r}`,
    actionLine: (u, s) => `U：升级 ${u}g   S：出售 ${s}g`,
    towerHeading: name => `${name}`,
    buildHint: '点击基座建塔,按键 1/2/3 切换。',
    remaining: n => `剩余 ${n}`,
    enemiesNext: n => `下一波 ${n} 个敌人`,
    coreDestroyed: '核心被摧毁',
    scoreLine: s => `分数 ${s}`,
  },
};

function t(key, ...args) {
  const v = TEXT[currentLang][key] ?? TEXT.en[key] ?? key;
  return typeof v === 'function' ? v(...args) : v;
}
function towerName(id) { return t('tName_' + id); }
function towerDesc(id) { return t('tDesc_' + id); }

function applyStaticText() {
  document.documentElement.lang = currentLang === 'zh' ? 'zh-CN' : 'en';
  document.querySelectorAll('[data-i18n]').forEach(el => { el.textContent = t(el.dataset.i18n); });
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
