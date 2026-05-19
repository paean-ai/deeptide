// Pixel Color Cascade - localization (English / 中文)
const LANG_KEY = 'pixel-color-cascade-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'COLOR CASCADE',
    subtitle: 'Flood the whole board into one colour.',
    howto: 'Tap a colour (or any tile) — your top-left region floods to it. Clear the board before the moves run out.',
    play: 'PLAY', levelSelect: 'LEVELS', level: 'Level',
    moves: 'Moves', par: 'Par', menu: 'Menu', retry: 'Retry', next: 'NEXT',
    win: 'BOARD CLEARED!', perfect: 'PERFECT!', failed: 'OUT OF MOVES',
    failMsg: 'The board is still a mess — give it another try.',
    starLine: (m, p) => `Cleared in ${m} moves (par ${p})`,
    locked: 'Locked', best: 'Best',
  },
  zh: {
    title: '色彩蔓延',
    subtitle: '把整个棋盘染成同一种颜色。',
    howto: '点击一种颜色(或任意方块),左上角的区域就会蔓延成该色。在步数用尽前清空棋盘。',
    play: '开始', levelSelect: '选关', level: '关卡',
    moves: '步数', par: '标准', menu: '菜单', retry: '重试', next: '下一关',
    win: '棋盘清空！', perfect: '完美！', failed: '步数耗尽',
    failMsg: '棋盘还是一团乱 —— 再试一次吧。',
    starLine: (m, p) => `用 ${m} 步清空(标准 ${p})`,
    locked: '未解锁', best: '最佳',
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
