// English / 中文 strings for Pixel Thermometers.

const STRINGS = {
  en: {
    title: 'PIXEL THERMOMETERS',
    subtitle: 'Fill each thermometer from its bulb',
    start: 'Start',
    levelSelect: 'Choose puzzle',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Solved!',
    next: 'Next puzzle',
    retry: 'Retry',
    timeStr: 'Time',
    mercury: 'MERCURY',
    rulesTxt1: 'Tap a cell to fill its thermometer up to that point.',
    rulesTxt2: 'Mercury always rises from the bulb - no gaps.',
    rulesTxt3: 'Match every row and column count to solve.',
  },
  zh: {
    title: '像素温度计',
    subtitle: '从灯泡端为每支温度计注液',
    start: '开始',
    levelSelect: '选择关卡',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '已解开！',
    next: '下一关',
    retry: '重玩',
    timeStr: '用时',
    mercury: '水银',
    rulesTxt1: '点格子，把温度计注到该处。',
    rulesTxt2: '水银总是从灯泡端升起，中间不能断。',
    rulesTxt3: '让每行每列的数字都对上即可通关。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-thermometers:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-thermometers:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
