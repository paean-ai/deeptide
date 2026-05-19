// Pixel Trail - English / Chinese strings.

const I18N = {
  en: {
    title: 'TRAIL',
    tagline: 'Draw the trail from 1 to the end — step by neighbouring step',
    play: 'PLAY',
    pick: 'SELECT A PATH',
    level: 'PATH',
    menu: 'MENU', restart: 'RESTART',
    win: 'TRAIL COMPLETE!',
    winLine: 'Every cell visited in order.',
    next: 'NEXT',
    step: 'NEXT',
    howto: 'Tap a cell next to the last filled one — it takes the next number in sequence. Hit every cell exactly once, matching the revealed clues, to complete the trail. Tap any earlier cell to backtrack.',
  },
  zh: {
    title: '链路',
    tagline: '从 1 开始铺设链路——一步一邻格',
    play: '开始',
    pick: '选择链路',
    level: '链路',
    menu: '菜单', restart: '重来',
    win: '链路完成！',
    winLine: '每格都按顺序走过。',
    next: '下一关',
    step: '下一步',
    howto: '点击与当前末端相邻的格子——它会自动获得下一个数字。每格只能走一次，并匹配所有已揭示的提示。点击之前走过的格子可以回退。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-trail-lang');
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
    try { localStorage.setItem('pixel-trail-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}
