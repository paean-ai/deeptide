// Pixel Flow - localization (English / 中文)
const LANG_KEY = 'pixel-flow-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'PIXEL FLOW',
    subtitle: 'Link every pair of dots. Fill the whole grid.',
    howto: 'Drag from a coloured dot to its twin without crossing other pipes. Cover every cell for a perfect clear.',
    play: 'PLAY', levelSelect: 'LEVELS', menu: 'MENU', retry: 'RETRY', next: 'NEXT',
    level: 'LEVEL', pairs: 'PAIRS',
    win: 'FLOW COMPLETE!', perfect: 'PERFECT FLOW!',
    winLine: pct => `Grid filled ${pct}%`,
    locked: 'LOCKED',
  },
  zh: {
    title: '像素连线',
    subtitle: '连接每一对圆点,填满整个网格。',
    howto: '从一个彩色圆点拖向它的同色伙伴,不要与其他管线交叉。填满每个格子可获得完美通关。',
    play: '开始', levelSelect: '选关', menu: '菜单', retry: '重玩', next: '下一关',
    level: '关卡', pairs: '配对',
    win: '连线完成！', perfect: '完美连线！',
    winLine: pct => `网格填充 ${pct}%`,
    locked: '未解锁',
  },
};

function t(key, ...args) {
  const v = TEXT[currentLang][key] ?? TEXT.en[key] ?? key;
  return typeof v === 'function' ? v(...args) : v;
}
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
