// Pixel Shikaku - English / Chinese strings.

const I18N = {
  en: {
    title: 'SHIKAKU',
    tagline: 'Carve the grid into rectangles — area equals the number',
    play: 'PLAY',
    pick: 'SELECT A PLOT',
    level: 'PLOT',
    menu: 'MENU', restart: 'RESTART',
    win: 'PLOT CLAIMED!',
    winLine: 'Every number housed in its rectangle.',
    next: 'NEXT',
    placed: 'PLACED',
    howto: 'Drag from one corner to the opposite corner to outline a rectangle. Each rectangle must contain exactly one number, and its area must equal that number. Tap an existing rectangle to remove it. Cover the whole grid to win.',
  },
  zh: {
    title: '数方格',
    tagline: '把网格切成矩形——面积等于数字',
    play: '开始',
    pick: '选择地块',
    level: '地块',
    menu: '菜单', restart: '重来',
    win: '地块完成！',
    winLine: '每个数字都收在自己的矩形里。',
    next: '下一关',
    placed: '已放置',
    howto: '从一个角拖到对角即可画出矩形。每个矩形必须恰好包含一个数字，且面积等于该数字。点击已放置的矩形可移除。把整张网格覆盖即可通关。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-shikaku-lang');
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
    try { localStorage.setItem('pixel-shikaku-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}
