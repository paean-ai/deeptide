// English / 中文 strings for Pixel Woody.

const STRINGS = {
  en: {
    title: 'PIXEL WOODY',
    subtitle: 'Drop blocks · fill rows or columns · clear lines',
    start: 'Start',
    backToMenu: 'Menu',
    newGame: 'New game',
    over: 'No more fits',
    score: 'Score',
    high: 'Best',
    placed: 'Placed',
    rulesTxt1: 'Tap a tray piece to pick it up; tap a board cell to drop it there.',
    rulesTxt2: 'Filling a full row OR column clears it; the streak bonus stacks.',
    rulesTxt3: 'When none of the three tray pieces fit anywhere, the run ends.',
  },
  zh: {
    title: '像素木块',
    subtitle: '拖入木块 · 填满整行整列 · 一次清除',
    start: '开始',
    backToMenu: '菜单',
    newGame: '新游戏',
    over: '已无可放',
    score: '分数',
    high: '最佳',
    placed: '已放',
    rulesTxt1: '点击托盘中的方块拾起，再点击棋盘格子放置。',
    rulesTxt2: '填满任意整行或整列即可消除，连消有奖。',
    rulesTxt3: '当托盘的三块都无处可放时，本局结束。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-woody:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-woody:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
