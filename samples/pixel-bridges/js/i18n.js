// Pixel Bridges - English / Chinese strings.

const I18N = {
  en: {
    title: 'BRIDGES',
    tagline: 'Link every island — match its number',
    play: 'PLAY',
    pick: 'SELECT A PUZZLE',
    level: 'ISLE',
    menu: 'MENU', restart: 'RESTART',
    win: 'CONNECTED!',
    winLine: 'Every island is linked.',
    next: 'NEXT',
    howto: 'Tap between two islands to lay a bridge; tap again for a double, once more to clear. Each number is how many bridges that island needs.',
  },
  zh: {
    title: '数桥',
    tagline: '连通每座岛——对上它的数字',
    play: '开始',
    pick: '选择谜题',
    level: '岛屿',
    menu: '菜单', restart: '重来',
    win: '全部连通！',
    winLine: '每座岛都连上了。',
    next: '下一关',
    howto: '在两岛之间点击架桥，再点变双桥，再点清除。数字表示该岛需要的桥数。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-bridges-lang');
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
    try { localStorage.setItem('pixel-bridges-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}
