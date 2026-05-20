// English / 中文 strings for Pixel Futoshiki.

const STRINGS = {
  en: {
    title: 'PIXEL FUTOSHIKI',
    subtitle: 'Latin square · inequalities · zero clues',
    start: 'Start',
    levelSelect: 'Choose ridge',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Solved!',
    next: 'Next ridge',
    retry: 'Retry',
    timeStr: 'Time',
    erase: 'Erase',
    rulesTxt1: 'Tap a cell, then tap a digit (1..n) on the number pad.',
    rulesTxt2: 'Each row and column needs every digit exactly once.',
    rulesTxt3: 'The > and < signs between cells must always hold.',
  },
  zh: {
    title: '像素不等数独',
    subtitle: '拉丁方 · 大小不等号 · 零提示',
    start: '开始',
    levelSelect: '选择关卡',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '完成！',
    next: '下一关',
    retry: '再来',
    timeStr: '用时',
    erase: '擦除',
    rulesTxt1: '先点格子再点数字键 (1..n)。',
    rulesTxt2: '每行每列恰好包含 1..n 各一次。',
    rulesTxt3: '格子之间的 > / < 不等号必须始终成立。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-futoshiki:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-futoshiki:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
