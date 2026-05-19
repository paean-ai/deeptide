// Pixel Unblock - localization (English / 中文)
const LANG_KEY = 'pixel-unblock-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'UNBLOCK',
    subtitle: 'Slide the blocks. Free the red one.',
    howto: 'Drag a block along its lane. Clear a path so the red block can slide out the right side.',
    play: 'PLAY', levelSelect: 'LEVELS', menu: 'MENU', retry: 'RETRY', next: 'NEXT',
    level: 'LEVEL',
    win: 'BLOCK FREED!', perfect: 'PERFECT ROUTE!',
    winLine: (mv, par) => `${mv} moves · par ${par}`,
    locked: 'LOCKED',
  },
  zh: {
    title: '华容方块',
    subtitle: '滑动方块,放出红色那一个。',
    howto: '沿着滑道拖动方块,清出通道,让红色方块从右侧滑出。',
    play: '开始', levelSelect: '选关', menu: '菜单', retry: '重玩', next: '下一关',
    level: '关卡',
    win: '方块脱困！', perfect: '完美路线！',
    winLine: (mv, par) => `${mv} 步 · 标准 ${par}`,
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
