// English / 中文 strings for Pixel Star Battle.

const STRINGS = {
  en: {
    title: 'PIXEL STAR BATTLE',
    subtitle: 'K stars per row · per column · per region · no touching',
    start: 'Start',
    levelSelect: 'Choose sky',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Aligned!',
    next: 'Next sky',
    retry: 'Retry',
    timeStr: 'Time',
    rulesTxt1: 'Tap a cell to cycle: blank → star → X (eliminated) → blank.',
    rulesTxt2: 'Each row, column, and outlined region needs exactly K stars.',
    rulesTxt3: 'Stars cannot touch each other - even diagonally.',
  },
  zh: {
    title: '像素星阵',
    subtitle: '每行 / 每列 / 每区都放 K 颗星 · 星不相邻',
    start: '开始',
    levelSelect: '选择夜空',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '对齐！',
    next: '下一片',
    retry: '再来',
    timeStr: '用时',
    rulesTxt1: '点格子循环：空 → 星 → ✕（排除）→ 空。',
    rulesTxt2: '每行、每列、每个粗框区都恰好放 K 颗星。',
    rulesTxt3: '相邻（含斜方向）的格子不能都是星。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-starbattle:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-starbattle:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
