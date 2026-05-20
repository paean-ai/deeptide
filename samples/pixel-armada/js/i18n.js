// English / 中文 strings for Pixel Armada.

const STRINGS = {
  en: {
    title: 'PIXEL ARMADA',
    subtitle: 'Battleship Solitaire · place the fleet, no ships touching',
    start: 'Start',
    levelSelect: 'Choose chart',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Fleet found!',
    next: 'Next chart',
    retry: 'Retry',
    timeStr: 'Time',
    fleet: 'Fleet',
    rulesTxt1: 'Tap a cell: blank → ship → water → blank.',
    rulesTxt2: 'Row / column numbers count the ship cells in that line.',
    rulesTxt3: 'Ships are straight; no two ships may touch — even diagonally.',
  },
  zh: {
    title: '像素舰队',
    subtitle: '战舰拼图 · 摆出舰队，舰只之间不相邻',
    start: '开始',
    levelSelect: '选择海图',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '舰队就位！',
    next: '下一张',
    retry: '再来',
    timeStr: '用时',
    fleet: '舰队',
    rulesTxt1: '点格子循环：空 → 舰 → 海水 → 空。',
    rulesTxt2: '行/列数字表示该行/列的舰格数。',
    rulesTxt3: '舰只为直线；任意两舰不能相邻，含斜方向。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-armada:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-armada:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
