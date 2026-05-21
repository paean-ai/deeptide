// English / 中文 strings for Pixel Slant.

const STRINGS = {
  en: {
    title: 'PIXEL SLANT',
    subtitle: 'Slash every cell · no loops',
    start: 'Start',
    levelSelect: 'Choose puzzle',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'No loops left!',
    next: 'Next puzzle',
    retry: 'Retry',
    timeStr: 'Time',
    rulesTxt1: 'Tap a cell to cycle: blank → \\ → / → blank.',
    rulesTxt2: 'A number = diagonals touching that lattice point.',
    rulesTxt3: 'Fill every cell — the slashes must form no loop.',
  },
  zh: {
    title: '像素斜线',
    subtitle: '为每格画斜线 · 不能成环',
    start: '开始',
    levelSelect: '选择关卡',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '无环达成！',
    next: '下一关',
    retry: '重玩',
    timeStr: '用时',
    rulesTxt1: '点格子循环：空 → \\ → / → 空。',
    rulesTxt2: '数字 = 连到该格点的斜线条数。',
    rulesTxt3: '填满每一格——斜线不能围出任何环。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-slant:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-slant:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
