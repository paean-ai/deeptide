// Pixel Glacier - English / Chinese strings.

const I18N = {
  en: {
    title: 'GLACIER',
    tagline: 'Slide across the ice — stop only at the rocks',
    play: 'PLAY',
    pick: 'SELECT A FLOE',
    level: 'FLOE',
    menu: 'MENU', restart: 'RESTART',
    win: 'ESCAPED!', perfect: 'FLAWLESS!',
    winLine: (mv, par) => mv + ' slides — par ' + par + '.',
    next: 'NEXT',
    moves: 'SLIDES', par: 'PAR',
    howto: 'Tap a direction and the explorer slides until a rock or the edge stops them. Reach the glowing exit — in par slides for the best rating.',
  },
  zh: {
    title: '冰川',
    tagline: '在冰面上滑行——只有岩石能让你停下',
    play: '开始',
    pick: '选择冰原',
    level: '冰原',
    menu: '菜单', restart: '重来',
    win: '逃脱了！', perfect: '完美！',
    winLine: (mv, par) => mv + ' 次滑行 — 标准 ' + par + ' 次。',
    next: '下一关',
    moves: '滑行', par: '标准',
    howto: '点击一个方向，探险者会一直滑到岩石或边缘才停下。抵达发光的出口——用标准次数完成可得最高评价。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-glacier-lang');
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
    try { localStorage.setItem('pixel-glacier-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}
