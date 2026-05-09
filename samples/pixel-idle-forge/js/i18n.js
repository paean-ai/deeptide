const LANG_KEY = 'starforge-idle-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    subtitle: 'Pixel star forge · infinite idle upgrades',
    reset: 'Reset',
    dust: 'Stardust',
    alloy: 'Alloy',
    core: 'Core',
    prestige: 'Prestige',
    multiplier: 'Total Multiplier',
    mine: 'Mine Stardust',
    click: 'Click',
    auto: 'Auto',
    upgrades: 'Upgrades',
    machines: 'Machines',
    relics: 'Relics',
    prestigeTitle: 'Starforge Recast',
    prestigeReady: gain => `Recast now for ${gain} prestige and permanent production.`,
    prestigeLocked: total => `Reach ${total}/50K total stardust to unlock the first recast.`,
    recast: 'Recast',
    langToggle: '中文',
    resetConfirm: 'Clear the Starforge Idle save?',
    resetDone: 'Save reset',
    offline: minutes => `Offline for ${minutes} minutes. Earnings claimed.`,
    leveled: (name, level) => `${name} reached Lv.${level}`,
    recastNeed: 'Reach at least 50K total stardust before recasting',
    recastConfirm: gain => `Recasting clears this run's resources and normal upgrades for ${gain} prestige. Continue?`,
    recastGain: gain => `Gained ${gain} prestige`,
    costDust: 'Stardust',
    costAlloy: 'Alloy',
    costCore: 'Core',
  },
  zh: {
    subtitle: '像素星炉 · 无限放置升级',
    reset: '重置',
    dust: '星尘',
    alloy: '合金',
    core: '核心',
    prestige: '声望',
    multiplier: '总倍率',
    mine: '开采星尘',
    click: '点击',
    auto: '自动',
    upgrades: '升级',
    machines: '机器',
    relics: '神器',
    prestigeTitle: '星炉重铸',
    prestigeReady: gain => `现在重铸可获得 ${gain} 声望，永久提升后续产出。`,
    prestigeLocked: total => `累计 ${total}/50K 星尘后解锁首次重铸。`,
    recast: '重铸',
    langToggle: 'English',
    resetConfirm: '确认清空 Starforge Idle 存档？',
    resetDone: '存档已重置',
    offline: minutes => `离线 ${minutes} 分钟，收益已结算`,
    leveled: (name, level) => `${name} 升至 Lv.${level}`,
    recastNeed: '至少累计 50K 星尘后可重铸',
    recastConfirm: gain => `重铸会清空本轮资源和普通升级，获得 ${gain} 声望。继续？`,
    recastGain: gain => `获得 ${gain} 声望`,
    costDust: '星尘',
    costAlloy: '合金',
    costCore: '核心',
  },
};

function t(key, ...args) {
  const value = TEXT[currentLang][key] ?? TEXT.en[key] ?? key;
  return typeof value === 'function' ? value(...args) : value;
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
