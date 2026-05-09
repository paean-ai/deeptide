const LANG_KEY = 'neon-rift-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    stage: 'Stage',
    chips: 'Chips',
    overclock: 'Skill Overclock',
    overclockSub: 'Choose one upgrade. Levels have no cap.',
    stageDone: 'Stage Cleared',
    riftRebuild: 'The rift is rebuilding.',
    nextStage: 'Next Stage',
    runEnded: 'Run Terminated',
    restart: 'Restart',
    langToggle: '中文',
    stagePopup: n => `Stage ${n}`,
    stageTitle: n => `Stage ${n} Cleared`,
    stageCopy: level => `The next rift is stronger. Current level: ${level}.`,
    gameoverCopy: (stage, level, coins) => `Reached stage ${stage}, level ${level}, collected ${coins} chips.`,
  },
  zh: {
    stage: '关卡',
    chips: '晶片',
    overclock: '技能超频',
    overclockSub: '选择一项强化，等级无上限',
    stageDone: '关卡完成',
    riftRebuild: '裂隙正在重组。',
    nextStage: '进入下一关',
    runEnded: '行动终止',
    restart: '重新开始',
    langToggle: 'English',
    stagePopup: n => `关卡 ${n}`,
    stageTitle: n => `关卡 ${n} 完成`,
    stageCopy: level => `下一段裂隙强度提升，当前等级 ${level}。`,
    gameoverCopy: (stage, level, coins) => `抵达关卡 ${stage}，等级 ${level}，收集 ${coins} 晶片。`,
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
