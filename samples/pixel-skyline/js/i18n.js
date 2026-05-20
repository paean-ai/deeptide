// Pixel Skyline - English / Chinese strings.

const I18N = {
  en: {
    title: 'SKYLINE',
    tagline: 'Build a city where every clue counts the visible towers',
    play: 'PLAY',
    pick: 'SELECT A SKYLINE',
    level: 'CITY',
    menu: 'MENU', restart: 'RESTART',
    win: 'SKYLINE COMPLETE!',
    winLine: 'Every clue is satisfied.',
    next: 'NEXT',
    howto: 'Tap a cell to cycle 1..n. Every row and column must hold each number once. A clue outside the grid is how many towers you see looking in - taller towers hide shorter ones behind them.',
  },
  zh: {
    title: '天际线',
    tagline: '建一座城市，让每个提示都对得上能看见的塔楼',
    play: '开始',
    pick: '选择天际线',
    level: '城市',
    menu: '菜单', restart: '重来',
    win: '天际线完成！',
    winLine: '所有提示都满足了。',
    next: '下一关',
    howto: '点击格子循环 1..n。每行每列各数字恰出现一次。边缘的提示是从外向内能看到的塔楼数——高的会挡住后面矮的。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-skyline-lang');
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
    try { localStorage.setItem('pixel-skyline-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}
