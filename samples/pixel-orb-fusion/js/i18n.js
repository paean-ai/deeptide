// Pixel Orb Fusion - localization (English / 中文)
const LANG_KEY = 'pixel-orb-fusion-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'ORB FUSION',
    subtitle: 'Slide the orbs. Fuse them. Reach 2048.',
    play: 'PLAY', howto: 'Arrows / WASD or swipe to slide every orb. Equal orbs fuse.',
    score: 'Score', best: 'Best',
    undo: 'Undo', restart: 'New', menu: 'Menu',
    paused: 'PAUSED', resume: 'Resume',
    gameOver: 'GRID LOCKED', finalScore: s => `Score: ${s}`,
    bestScore: s => `Best: ${s}`, again: 'TRY AGAIN', keepGoing: 'KEEP FUSING',
    reached2048: 'THE 2048 ORB!', win2048: 'You forged the 2048 orb — keep going for a record.',
  },
  zh: {
    title: '元珠融合',
    subtitle: '滑动元珠，融合升级，合成 2048。',
    play: '开始', howto: '方向键 / WASD 或滑动来移动所有元珠，相同元珠融合。',
    score: '分数', best: '最高',
    undo: '撤销', restart: '新局', menu: '菜单',
    paused: '已暂停', resume: '继续',
    gameOver: '棋盘锁死', finalScore: s => `分数：${s}`,
    bestScore: s => `最高：${s}`, again: '再来一局', keepGoing: '继续融合',
    reached2048: '2048 元珠！', win2048: '你合成了 2048 元珠 —— 继续冲击纪录吧。',
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
