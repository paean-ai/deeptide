// Pixel Slide - localization (English / 中文)
const LANG_KEY = 'pixel-slide-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'SLIDE',
    subtitle: 'Slide the tiles back into order.',
    howto: 'Tap a tile next to the gap to slide it across. Restore the numbers — and the colour gradient — to win.',
    play: 'PLAY', levelSelect: 'LEVELS', menu: 'MENU', retry: 'RETRY', next: 'NEXT',
    level: 'LEVEL',
    win: 'SOLVED!', perfect: 'CLEAN SOLVE!',
    winLine: (mv, par) => `${mv} moves · par ${par}`,
    locked: 'LOCKED',
  },
  zh: {
    title: '滑块拼图',
    subtitle: '把数字方块滑回正确顺序。',
    howto: '点击空格旁边的方块即可把它滑过去。还原数字与颜色渐变即获胜。',
    play: '开始', levelSelect: '选关', menu: '菜单', retry: '重玩', next: '下一关',
    level: '关卡',
    win: '完成！', perfect: '漂亮还原！',
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
