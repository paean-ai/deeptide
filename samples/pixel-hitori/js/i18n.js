// Pixel Hitori - English / Chinese strings.

const I18N = {
  en: {
    title: 'HITORI',
    tagline: 'Shade out the duplicates — never two side by side',
    play: 'PLAY',
    pick: 'SELECT A GRID',
    level: 'GRID',
    menu: 'MENU', restart: 'RESTART',
    win: 'CLEARED!',
    winLine: 'Every row and column is clean.',
    next: 'NEXT',
    howto: 'Shade cells so no number repeats in any row or column. Shaded cells may not touch orthogonally, and every unshaded cell must connect. Tap to cycle blank → shaded → marked-open.',
  },
  zh: {
    title: '独伊',
    tagline: '涂掉重复——黑格不可相邻',
    play: '开始',
    pick: '选择棋盘',
    level: '棋盘',
    menu: '菜单', restart: '重来',
    win: '通关！',
    winLine: '每行每列都纯净了。',
    next: '下一关',
    howto: '涂黑某些格，使每行每列不出现重复数字。黑格不可正交相邻，且白格须连通成一片。点击循环 留白 → 涂黑 → 标白。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-hitori-lang');
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
    try { localStorage.setItem('pixel-hitori-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}
