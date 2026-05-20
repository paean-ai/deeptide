// English / 中文 strings for Pixel Aquarium.

const STRINGS = {
  en: {
    title: 'PIXEL AQUARIUM',
    subtitle: 'Each tank fills bottom-up · row + column counts pin the levels',
    start: 'Start',
    levelSelect: 'Choose tank',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Levels solved!',
    next: 'Next tank',
    retry: 'Retry',
    timeStr: 'Time',
    rulesTxt1: 'Tap a cell to cycle: blank → water → air → blank.',
    rulesTxt2: 'Row / column numbers count the water cells in that line.',
    rulesTxt3: 'Inside a tank water fills bottom-up — no air below a water cell.',
  },
  zh: {
    title: '像素水族箱',
    subtitle: '每个水箱自下向上注水 · 行列数字定位水位',
    start: '开始',
    levelSelect: '选择水箱',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '水位推定！',
    next: '下一缸',
    retry: '再来',
    timeStr: '用时',
    rulesTxt1: '点格子循环：空 → 水 → 气 → 空。',
    rulesTxt2: '行/列数字表示该行/列的水格数。',
    rulesTxt3: '同一水箱里水从底往上盈 — 水下面不能有气。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-aquarium:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-aquarium:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
