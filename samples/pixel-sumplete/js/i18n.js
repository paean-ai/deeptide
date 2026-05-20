// English / 中文 strings for Pixel Sumplete.

const STRINGS = {
  en: {
    title: 'PIXEL SUMPLETE',
    subtitle: 'Keep or delete · row + column sums hit the targets',
    start: 'Start',
    levelSelect: 'Choose grid',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Sums match!',
    next: 'Next grid',
    retry: 'Retry',
    timeStr: 'Time',
    rulesTxt1: 'Tap a cell to cycle: keep → delete → undecided → keep.',
    rulesTxt2: 'Sum of KEPT cells in each row must equal the row target.',
    rulesTxt3: 'Same rule for every column.',
  },
  zh: {
    title: '像素求和',
    subtitle: '保留或删除 · 行列和命中目标',
    start: '开始',
    levelSelect: '选择关卡',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '求和匹配！',
    next: '下一关',
    retry: '再来',
    timeStr: '用时',
    rulesTxt1: '点格子循环：保留 → 删除 → 未定 → 保留。',
    rulesTxt2: '每行保留格的和应等于该行目标。',
    rulesTxt3: '列同理。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-sumplete:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-sumplete:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
