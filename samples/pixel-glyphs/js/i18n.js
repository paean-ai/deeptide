// Pixel Glyphs - English / Chinese strings.

const I18N = {
  en: {
    title: 'GLYPHS',
    tagline: 'Light every glyph to break the lock',
    play: 'PLAY',
    pick: 'SELECT A LOCK',
    level: 'LOCK',
    menu: 'MENU', restart: 'RESTART',
    win: 'UNLOCKED!',
    perfect: 'FLAWLESS!',
    winLine: (mv, par) => 'Solved in ' + mv + ' — par ' + par + '.',
    next: 'NEXT',
    moves: 'MOVES', par: 'PAR',
    howto: 'Tap a glyph to flip it and its neighbours. Light every glyph at once to open the lock — fewest presses wins the stars.',
  },
  zh: {
    title: '符印',
    tagline: '点亮所有符文即可解开封印',
    play: '开始',
    pick: '选择封印',
    level: '封印',
    menu: '菜单', restart: '重来',
    win: '解开了！',
    perfect: '完美！',
    winLine: (mv, par) => mv + ' 步解开 — 标准 ' + par + ' 步。',
    next: '下一关',
    moves: '步数', par: '标准',
    howto: '点击符文会翻转它及其相邻符文。让所有符文同时点亮即可解锁——步数越少星级越高。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-glyphs-lang');
  if (s === 'en' || s === 'zh') lang = s;
} catch (e) { /* ignore */ }

function t(key, ...args) {
  const v = I18N[lang][key];
  return typeof v === 'function' ? v(...args) : v;
}

function setupLanguageToggle(onChange) {
  const btn = document.getElementById('btn-lang');
  if (!btn) return;
  const sync = () => { btn.textContent = lang === 'en' ? '中文' : 'EN'; };
  sync();
  btn.addEventListener('click', () => {
    lang = lang === 'en' ? 'zh' : 'en';
    try { localStorage.setItem('pixel-glyphs-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}
