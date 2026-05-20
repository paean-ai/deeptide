// English / 中文 strings for Pixel Kakurasu.

const STRINGS = {
  en: {
    title: 'PIXEL KAKURASU',
    subtitle: 'Index sums · shaded cells must hit every row + column target',
    start: 'Start',
    levelSelect: 'Choose grid',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Sums aligned!',
    next: 'Next grid',
    retry: 'Retry',
    timeStr: 'Time',
    rulesTxt1: 'Tap a cell to cycle: undecided → shaded → empty → undecided.',
    rulesTxt2: 'Row score = sum of column indices of shaded cells (col index = 1..n).',
    rulesTxt3: 'Column score = sum of row indices of shaded cells. Hit every target.',
  },
  zh: {
    title: '像素 Kakurasu',
    subtitle: '索引求和 · 涂色格命中每个行列目标',
    start: '开始',
    levelSelect: '选择关卡',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '行列对齐！',
    next: '下一关',
    retry: '再来',
    timeStr: '用时',
    rulesTxt1: '点格子循环：未定 → 涂色 → 留空 → 未定。',
    rulesTxt2: '行得分 = 该行涂色格的列号之和（列号 1..n）。',
    rulesTxt3: '列得分 = 该列涂色格的行号之和。两者都要命中目标。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-kakurasu:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-kakurasu:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
