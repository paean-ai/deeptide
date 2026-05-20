// English / 中文 strings for Pixel Magnets.

const STRINGS = {
  en: {
    title: 'PIXEL MAGNETS',
    subtitle: 'Charge each 1×2 magnet · row + col counts pin the layout',
    start: 'Start',
    levelSelect: 'Choose rig',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Polarised!',
    next: 'Next rig',
    retry: 'Retry',
    timeStr: 'Time',
    rulesTxt1: 'Tap a cell to cycle: blank → + → − → empty (X) → blank.',
    rulesTxt2: 'Each magnet is two cells: leave both blank OR set +/− on its ends.',
    rulesTxt3: 'Same-charge cells can never share an edge.',
  },
  zh: {
    title: '像素磁阵',
    subtitle: '为每块 1×2 磁铁定极性 · 行列数字定布局',
    start: '开始',
    levelSelect: '选择磁台',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '极性确定！',
    next: '下一台',
    retry: '再来',
    timeStr: '用时',
    rulesTxt1: '点格子循环：空 → + → − → 空白(X) → 空。',
    rulesTxt2: '每块磁铁占两格：要么都空，要么两端为 + / −。',
    rulesTxt3: '相同极性的格子不能上下左右相邻。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-magnets:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-magnets:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
