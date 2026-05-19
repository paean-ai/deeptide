// Pixel Mosaic - English / Chinese strings.

const I18N = {
  en: {
    title: 'MOSAIC',
    tagline: 'Every cell counts its 3×3 — shade to match',
    play: 'PLAY',
    pick: 'SELECT A PICTURE',
    level: 'PICTURE',
    menu: 'MENU', restart: 'RESTART',
    win: 'PICTURE COMPLETE!',
    winLine: 'Every clue satisfied.',
    next: 'NEXT',
    howto: 'Each cell\'s number is how many cells in its 3×3 neighbourhood (including itself) should be filled. Tap a cell to cycle filled → marked-empty → blank. Satisfy every number to reveal the picture.',
  },
  zh: {
    title: '马赛克',
    tagline: '每格统计 3×3 邻域——按数字涂色',
    play: '开始',
    pick: '选择图画',
    level: '图画',
    menu: '菜单', restart: '重来',
    win: '图画完成！',
    winLine: '每个数字都被满足。',
    next: '下一关',
    howto: '每格的数字表示其 3×3 邻域（含自身）应被涂黑的格数。点击格子在 涂黑 → 标空 → 留白 之间循环。满足所有数字即显现图画。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-mosaic-lang');
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
    try { localStorage.setItem('pixel-mosaic-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}
