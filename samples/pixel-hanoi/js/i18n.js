// English / 中文 strings for Pixel Hanoi.

const STRINGS = {
  en: {
    title: 'PIXEL HANOI',
    subtitle: 'Move every disk to the right peg · never a larger on a smaller',
    start: 'Start',
    levelSelect: 'Choose tower',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Tower rebuilt!',
    next: 'Next tower',
    retry: 'Retry',
    undo: 'Undo',
    restart: 'Restart',
    moves: 'Moves',
    par: 'Par',
    best: 'Best',
    disks: 'Disks',
    rulesTxt1: 'Tap a peg to pick up its top disk; tap another peg to drop.',
    rulesTxt2: 'You may never place a larger disk on top of a smaller one.',
    rulesTxt3: 'Solve in the optimal 2^N - 1 moves to earn three stars.',
  },
  zh: {
    title: '像素汉诺塔',
    subtitle: '把所有圆盘移到右柱 · 大盘永远不能压小盘',
    start: '开始',
    levelSelect: '选择塔',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '汉诺塔重建！',
    next: '下一关',
    retry: '再来',
    undo: '撤销',
    restart: '重开',
    moves: '步数',
    par: '标准',
    best: '最少',
    disks: '圆盘',
    rulesTxt1: '点击柱子拾起顶部圆盘，再点另一柱放下。',
    rulesTxt2: '永远不能把大盘压在小盘之上。',
    rulesTxt3: '用 2^N - 1 步的最优解通关即得三星。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-hanoi:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-hanoi:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
