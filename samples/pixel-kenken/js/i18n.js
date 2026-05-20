// English / 中文 strings for Pixel KenKen.

const STRINGS = {
  en: {
    title: 'PIXEL KENKEN',
    subtitle: 'Latin square · cage arithmetic · no extra clues',
    start: 'Start',
    levelSelect: 'Choose grid',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Solved!',
    next: 'Next grid',
    retry: 'Retry',
    timeStr: 'Time',
    erase: 'Erase',
    rulesTxt1: 'Tap a cell, then tap a digit (1..n) on the pad.',
    rulesTxt2: 'Each row + column needs every digit exactly once.',
    rulesTxt3: 'Each outlined cage must satisfy its target + operator.',
  },
  zh: {
    title: '像素 KenKen',
    subtitle: '拉丁方·区域算式·零提示',
    start: '开始',
    levelSelect: '选择格盘',
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
    rulesTxt3: '每个粗框区域要符合其目标与运算符。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-kenken:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-kenken:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
