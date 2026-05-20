// Pixel Kuromasu - English / Chinese strings.

const I18N = {
  en: {
    title: 'KUROMASU',
    tagline: 'Shade cells so every number sees exactly its count',
    play: 'PLAY',
    pick: 'SELECT A GRID',
    level: 'GRID',
    menu: 'MENU', restart: 'RESTART',
    win: 'CLEARED!',
    winLine: 'Every clue sees the right count.',
    next: 'NEXT',
    howto: 'A number is how many cells (itself included) it can see in the four directions before a black cell or the grid edge. Tap a blank to cycle empty → black → marked-white. Black cells must not touch and every white cell must connect.',
  },
  zh: {
    title: '黑格谜',
    tagline: '将格子涂黑，使每个数字能看见的格数与它一致',
    play: '开始',
    pick: '选择棋盘',
    level: '棋盘',
    menu: '菜单', restart: '重来',
    win: '完成！',
    winLine: '每个提示看到的格数都对了。',
    next: '下一关',
    howto: '数字表示该格（含自身）四向能看到的格数，遇黑格或边缘即止。点击空格循环 空 → 黑 → 标白。黑格不可相邻，白格须连通成一片。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-kuromasu-lang');
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
    try { localStorage.setItem('pixel-kuromasu-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}
