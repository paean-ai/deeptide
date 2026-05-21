// English / 中文 strings for Pixel Dice.

const STRINGS = {
  en: {
    title: 'PIXEL DICE',
    subtitle: 'Roll the die · set every seal',
    start: 'Start',
    levelSelect: 'Choose puzzle',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'All seals set!',
    next: 'Next puzzle',
    retry: 'Retry',
    undo: 'Undo',
    restart: 'Restart',
    moves: 'Rolls',
    par: 'Par',
    rulesTxt1: 'Tap a cell beside the die to roll it there.',
    rulesTxt2: 'A seal is set when the die lands on it that number up.',
    rulesTxt3: 'Set every seal; match par for three stars.',
  },
  zh: {
    title: '像素骰子',
    subtitle: '滚动骰子 · 点亮所有印记',
    start: '开始',
    levelSelect: '选择关卡',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '全部点亮！',
    next: '下一关',
    retry: '重玩',
    undo: '撤销',
    restart: '重置',
    moves: '步数',
    par: '标准',
    rulesTxt1: '点击骰子旁的格子，把它滚过去。',
    rulesTxt2: '骰子滚到印记上、且该数字朝上时，印记被点亮。',
    rulesTxt3: '点亮全部印记；达到标准步数得三星。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-dice:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-dice:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
