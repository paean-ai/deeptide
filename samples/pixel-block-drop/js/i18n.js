// Pixel Block Drop - localization (English / 中文)
const LANG_KEY = 'pixel-block-drop-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'BLOCK DROP',
    subtitle: 'Drop the blocks, clear the lines.',
    howto: 'Move and rotate the falling piece to complete full rows. Hold a piece for later. Stack too high and it is over.',
    keys: 'Keys: ← → move · ↑ rotate · ↓ soft drop · Space hard drop · C hold',
    play: 'PLAY', again: 'PLAY AGAIN', menu: 'MENU',
    score: 'SCORE', level: 'LEVEL', lines: 'LINES', best: 'BEST',
    hold: 'HOLD', next: 'NEXT',
    gameOver: 'GAME OVER',
    finalScore: s => `Score: ${s}`,
    newBest: 'NEW BEST!',
  },
  zh: {
    title: '方块坠落',
    subtitle: '落下方块,消除整行。',
    howto: '移动并旋转下落的方块来填满整行。可暂存一个方块。堆得太高就结束了。',
    keys: '按键:← → 移动 · ↑ 旋转 · ↓ 软降 · 空格 硬降 · C 暂存',
    play: '开始', again: '再玩一次', menu: '菜单',
    score: '分数', level: '等级', lines: '行数', best: '最高',
    hold: '暂存', next: '下一个',
    gameOver: '游戏结束',
    finalScore: s => `分数：${s}`,
    newBest: '新纪录！',
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
