// Pixel Tents - English / Chinese strings.

const I18N = {
  en: {
    title: 'TENTS',
    tagline: 'Pitch a tent next to every tree',
    play: 'PLAY',
    pick: 'SELECT A FOREST',
    level: 'FOREST',
    menu: 'MENU', restart: 'RESTART',
    win: 'CAMP SET!',
    winLine: 'Every tree has its tent.',
    next: 'NEXT',
    howto: 'Tap a cell to cycle blank → tent → grass. Every tree needs one orthogonally-adjacent tent, every tent needs one orthogonally-adjacent tree, no two tents may touch (even diagonally), and the side numbers count the tents in each row / column.',
  },
  zh: {
    title: '帐篷',
    tagline: '为每棵树搭一顶帐篷',
    play: '开始',
    pick: '选择森林',
    level: '森林',
    menu: '菜单', restart: '重来',
    win: '扎营完成！',
    winLine: '每棵树都有它的帐篷。',
    next: '下一关',
    howto: '点击格子循环 空 → 帐篷 → 草。每棵树需要正交相邻一顶帐篷，每顶帐篷也需正交相邻一棵树；任意两顶帐篷不可相邻（含对角）；边上的数字是该行/列的帐篷数。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-tents-lang');
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
    try { localStorage.setItem('pixel-tents-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}
