// Pixel Dominosa - English / Chinese strings.

const I18N = {
  en: {
    title: 'DOMINOSA',
    tagline: 'Split the pips into one full domino set',
    play: 'PLAY',
    pick: 'SELECT A PUZZLE',
    level: 'FIELD',
    menu: 'MENU', restart: 'RESTART',
    win: 'COMPLETE!',
    winLine: 'Every domino placed once.',
    next: 'NEXT',
    left: 'LEFT',
    howto: 'Tap between two squares to lay a domino across them. Every domino from 0-0 up must appear exactly once, with no square left over.',
  },
  zh: {
    title: '多米诺莎',
    tagline: '把点数划分成完整的一副骨牌',
    play: '开始',
    pick: '选择谜题',
    level: '牌田',
    menu: '菜单', restart: '重来',
    win: '完成！',
    winLine: '每张骨牌都恰好用了一次。',
    next: '下一关',
    left: '剩余',
    howto: '在两格之间点击放置一张骨牌。从 0-0 起的每张骨牌都须恰好出现一次，不留空格。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-dominosa-lang');
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
    try { localStorage.setItem('pixel-dominosa-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}
