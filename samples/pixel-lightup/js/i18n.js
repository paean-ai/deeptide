// Pixel Light Up - English / Chinese strings.

const I18N = {
  en: {
    title: 'LIGHT UP',
    tagline: 'Light every cell — no bulb in another’s beam',
    play: 'PLAY',
    pick: 'SELECT A PUZZLE',
    level: 'ROOM',
    menu: 'MENU', restart: 'RESTART',
    win: 'ALL LIT!',
    winLine: 'Every cell is glowing.',
    next: 'NEXT',
    dark: 'DARK',
    howto: 'Tap a cell to place a bulb. A bulb lights its row and column until a wall. No bulb may shine on another, and a numbered wall needs exactly that many bulbs beside it.',
  },
  zh: {
    title: '点灯',
    tagline: '照亮每格——灯光不可互相照射',
    play: '开始',
    pick: '选择谜题',
    level: '房间',
    menu: '菜单', restart: '重来',
    win: '全部点亮！',
    winLine: '每一格都亮了。',
    next: '下一关',
    dark: '未照亮',
    howto: '点击格子放置灯泡。灯泡照亮所在行列直到墙壁。灯泡不能互相照射，带数字的墙旁须恰好有相应数量的灯泡。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-lightup-lang');
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
    try { localStorage.setItem('pixel-lightup-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}
