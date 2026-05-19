// Pixel Bullet Storm - localization (English / 中文)
const LANG_KEY = 'pixel-bullet-storm-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'BULLET STORM',
    subtitle: 'Weave through the storm. Trust your tiny core.',
    howto: 'Drag to fly — your ship rides above your finger. The bright dot is your only hitbox.',
    keys: 'Keys: arrows / WASD to move · Space to bomb',
    play: 'LAUNCH', again: 'RETRY', menu: 'MENU',
    score: 'SCORE', wave: 'WAVE',
    gameOver: 'SHIP DOWN',
    bossWarning: 'WARNING — BOSS',
    waveClear: 'WAVE CLEAR',
    finalLine: (sc, wv) => `Score ${sc} · reached wave ${wv}`,
    bestLine: sc => `Best ${sc}`,
    newBest: 'NEW BEST!',
  },
  zh: {
    title: '弹幕风暴',
    subtitle: '在弹雨中穿行,相信你那颗小小的核心。',
    howto: '拖动来飞行 —— 飞船会浮在手指上方。那个亮点才是你唯一的判定点。',
    keys: '按键:方向键 / WASD 移动 · 空格放炸弹',
    play: '起飞', again: '重来', menu: '菜单',
    score: '分数', wave: '波次',
    gameOver: '战机坠落',
    bossWarning: '警告 —— 头目',
    waveClear: '波次清空',
    finalLine: (sc, wv) => `得分 ${sc} · 抵达第 ${wv} 波`,
    bestLine: sc => `最高 ${sc}`,
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
