// English / 中文 strings for Pixel Binairo.

const STRINGS = {
  en: {
    title: 'PIXEL BINAIRO',
    subtitle: 'Fill 0s and 1s · balance every line · no three in a row',
    start: 'Start',
    levelSelect: 'Choose grid',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Grid balanced!',
    next: 'Next grid',
    retry: 'Retry',
    timeStr: 'Time',
    rulesTxt1: 'Tap a cell to cycle blank → 0 → 1 → blank.',
    rulesTxt2: 'No three of the same value in a row or column.',
    rulesTxt3: 'Each row and column needs equal 0s and 1s — and stays unique.',
  },
  zh: {
    title: '像素二进制',
    subtitle: '填入 0 和 1 · 每行每列平衡 · 不能三连',
    start: '开始',
    levelSelect: '选择棋盘',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '棋盘平衡！',
    next: '下一关',
    retry: '再来',
    timeStr: '用时',
    rulesTxt1: '点击格子循环：空 → 0 → 1 → 空。',
    rulesTxt2: '任意行或列都不能有三个相同的值相连。',
    rulesTxt3: '每行每列的 0 和 1 数量相等，且不能有重复的行或列。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-binairo:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-binairo:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
