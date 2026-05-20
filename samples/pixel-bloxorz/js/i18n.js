// English / 中文 strings for Pixel Bloxorz.

const STRINGS = {
  en: {
    title: 'PIXEL BLOXORZ',
    subtitle: 'Roll the 1x1x2 block · stand on the hole · do not fall off',
    start: 'Start',
    levelSelect: 'Choose level',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Standing on the hole!',
    lose: 'The block fell',
    next: 'Next level',
    retry: 'Retry',
    undo: 'Undo',
    restart: 'Restart',
    moves: 'Moves',
    par: 'Par',
    best: 'Best',
    rulesTxt1: 'Tap the edges (or arrow keys / WASD) to roll the block.',
    rulesTxt2: 'A weak tile holds the block lying flat — but not standing.',
    rulesTxt3: 'Reach the hole STANDING UP. Roll off the grid and the block falls.',
  },
  zh: {
    title: '像素方块滚滚',
    subtitle: '滚动 1x1x2 方块 · 立在洞口 · 别掉下去',
    start: '开始',
    levelSelect: '选择关卡',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '立在洞口！',
    lose: '方块掉下去了',
    next: '下一关',
    retry: '再来',
    undo: '撤销',
    restart: '重开',
    moves: '步数',
    par: '标准',
    best: '最少',
    rulesTxt1: '点击屏幕四周（或方向键 / WASD）滚动方块。',
    rulesTxt2: '脆弱地砖能托住躺着的方块——但承受不住竖立。',
    rulesTxt3: '必须竖立在洞口才胜利。滚出网格就掉下去。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-bloxorz:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-bloxorz:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
