// Pixel Bubble Pop - localization (English / 中文)
const LANG_KEY = 'pixel-bubble-pop-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'BUBBLE POP',
    subtitle: 'Aim, shoot, and burst clusters of three or more.',
    play: 'PLAY', howto: 'Aim with the mouse or finger, release to shoot. Match 3+ to pop.',
    score: 'Score', best: 'Best', shots: 'Shots',
    paused: 'PAUSED', resume: 'Resume', restart: 'Restart', menu: 'Menu',
    gameOver: 'BUBBLES OVERFLOW', finalScore: s => `Score: ${s}`,
    bestScore: s => `Best: ${s}`, again: 'POP AGAIN',
    cleared: 'BOARD CLEARED!',
  },
  zh: {
    title: '泡泡爆破',
    subtitle: '瞄准、发射,击爆三个及以上的同色泡泡。',
    play: '开始', howto: '用鼠标或手指瞄准,松开发射。三连以上即可爆破。',
    score: '分数', best: '最高', shots: '发射',
    paused: '已暂停', resume: '继续', restart: '重新开始', menu: '菜单',
    gameOver: '泡泡溢出', finalScore: s => `分数：${s}`,
    bestScore: s => `最高：${s}`, again: '再爆一局',
    cleared: '清空棋盘！',
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
