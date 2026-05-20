// English / 中文 strings for Pixel Norinori.

const STRINGS = {
  en: {
    title: 'PIXEL NORINORI',
    subtitle: 'Shade one domino per region · no two dominoes share an edge',
    start: 'Start',
    levelSelect: 'Choose region',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'All paired!',
    next: 'Next region',
    retry: 'Retry',
    timeStr: 'Time',
    rulesTxt1: 'Tap a cell to cycle: blank → shaded → empty-mark → blank.',
    rulesTxt2: 'Each thick-outlined region must hold exactly one 1×2 domino.',
    rulesTxt3: 'Dominoes from different regions cannot share an edge.',
  },
  zh: {
    title: '像素莲莲',
    subtitle: '每个区域刚好涂出一块多米诺·两块多米诺不能相邻',
    start: '开始',
    levelSelect: '选择关卡',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '全部配对！',
    next: '下一关',
    retry: '再来',
    timeStr: '用时',
    rulesTxt1: '点格子循环：空 → 涂黑 → 标白点 → 空。',
    rulesTxt2: '每个粗框区域恰好涂一块 1×2 多米诺。',
    rulesTxt3: '不同区域的多米诺不能上下左右相邻。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-norinori:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-norinori:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
