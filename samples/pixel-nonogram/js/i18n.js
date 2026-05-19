// Pixel Nonogram - localization (English / 中文)
const LANG_KEY = 'pixel-nonogram-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'NONOGRAM',
    subtitle: 'Read the clues. Uncover the hidden pixel art.',
    howto: 'Each number is a run of filled cells in that row or column. Fill the right cells to reveal the picture.',
    play: 'PLAY', levelSelect: 'LEVELS', menu: 'MENU', retry: 'RETRY', next: 'NEXT',
    level: 'LEVEL',
    modeFill: 'MODE: FILL', modeMark: 'MODE: MARK',
    win: 'PICTURE REVEALED!', perfect: 'FLAWLESS!',
    nameLine: n => `It's a ${n}!`,
    winLine: (tm, ms) => `Time ${tm} · ${ms} mistakes`,
    locked: 'LOCKED',
  },
  zh: {
    title: '数织',
    subtitle: '读懂线索,揭开隐藏的像素画。',
    howto: '每个数字代表该行或该列中一段连续的填充格。填对格子即可显出图案。',
    play: '开始', levelSelect: '选关', menu: '菜单', retry: '重玩', next: '下一关',
    level: '关卡',
    modeFill: '模式:填充', modeMark: '模式:标记',
    win: '图案显现！', perfect: '完美无瑕！',
    nameLine: n => `这是${n}！`,
    winLine: (tm, ms) => `用时 ${tm} · ${ms} 次失误`,
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
