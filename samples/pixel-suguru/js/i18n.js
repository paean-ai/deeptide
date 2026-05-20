// English / 中文 strings for Pixel Suguru.

const STRINGS = {
  en: {
    title: 'PIXEL SUGURU',
    subtitle: 'Fill each region with 1..n — no two equal digits touching',
    start: 'Start',
    levelSelect: 'Choose puzzle',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    erase: 'Erase',
    notes: 'Notes',
    notesOn: 'Notes ON',
    notesOff: 'Notes OFF',
    hint: 'Hint',
    reset: 'Reset',
    rules: 'Rules',
    rulesTxt1: 'Each outlined region of size n holds the digits 1..n.',
    rulesTxt2: 'The same digit can never touch itself, not even diagonally.',
    rulesTxt3: 'Tap a cell, then tap a digit. NOTES leaves a pencil mark.',
    win: 'Solved!',
    next: 'Next puzzle',
    retry: 'Retry',
    mistakes: 'Mistakes',
    timeStr: 'Time',
  },
  zh: {
    title: '像素数独岛',
    subtitle: '把每个区域填上 1..n，相同数字不能相邻（含斜角）',
    start: '开始',
    levelSelect: '选择关卡',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    erase: '擦除',
    notes: '草稿',
    notesOn: '草稿 开',
    notesOff: '草稿 关',
    hint: '提示',
    reset: '重置',
    rules: '规则',
    rulesTxt1: '每个粗框区域大小为 n，需填入 1..n 各一次。',
    rulesTxt2: '相同数字不能相邻，包括斜方向。',
    rulesTxt3: '先点格子再点数字；开启草稿可做小记号。',
    win: '完成！',
    next: '下一关',
    retry: '再来一次',
    mistakes: '错误',
    timeStr: '用时',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-suguru:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-suguru:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
