// Pixel Sudoku - localization (English / 中文)
const LANG_KEY = 'pixel-sudoku-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'SUDOKU',
    subtitle: 'Fill the grid — every row, column and box holds 1–9.',
    howto: 'Tap a cell, tap a number. Toggle NOTES for pencil marks. Every puzzle has exactly one solution.',
    play: 'PLAY', levelSelect: 'LEVELS', menu: 'MENU', retry: 'RETRY', next: 'NEXT',
    win: 'SOLVED!', perfect: 'FLAWLESS!',
    tiers: ['EASY', 'MEDIUM', 'HARD'],
    winLine: (tm, ms) => `Time ${tm} · ${ms} mistakes`,
    locked: 'LOCKED',
  },
  zh: {
    title: '数独',
    subtitle: '填满网格 —— 每行、每列、每宫都含 1–9。',
    howto: '点击格子,再点数字。开启「笔记」可做候选标记。每道题都只有唯一解。',
    play: '开始', levelSelect: '选关', menu: '菜单', retry: '重玩', next: '下一关',
    win: '完成！', perfect: '完美无误！',
    tiers: ['简单', '中等', '困难'],
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
