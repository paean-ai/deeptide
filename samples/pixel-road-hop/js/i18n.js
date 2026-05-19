// Pixel Road Hop - localization (English / 中文)
const LANG_KEY = 'pixel-road-hop-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'ROAD HOP',
    subtitle: 'Hop across roads and rivers — how far can you get?',
    howto: 'Swipe or tap to hop. Dodge the traffic, ride the logs, and never stop too long.',
    keys: 'Keys: arrows / WASD to hop',
    play: 'HOP', again: 'HOP AGAIN', menu: 'MENU',
    score: 'SCORE', best: 'BEST',
    gameOver: 'SPLAT!', drowned: 'GLUG GLUG', caught: 'TOO SLOW!',
    finalScore: s => `You hopped ${s} rows`,
    newBest: 'NEW BEST!',
  },
  zh: {
    title: '过马路',
    subtitle: '跳过马路和河流 —— 你能走多远?',
    howto: '滑动或点击来跳跃。躲开车流、踩着木头过河,别停太久。',
    keys: '按键:方向键 / WASD 跳跃',
    play: '开跳', again: '再跳一次', menu: '菜单',
    score: '分数', best: '最高',
    gameOver: '被撞扁了！', drowned: '咕嘟咕嘟', caught: '太慢啦！',
    finalScore: s => `你跳过了 ${s} 行`,
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
