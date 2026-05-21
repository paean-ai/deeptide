// English / 中文 strings for Pixel Numpath.

const STRINGS = {
  en: {
    title: 'PIXEL NUMPATH',
    subtitle: 'Trace 1 to N · one unbroken path',
    start: 'Start',
    levelSelect: 'Choose puzzle',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Path complete!',
    next: 'Next puzzle',
    retry: 'Retry',
    timeStr: 'Time',
    rulesTxt1: 'Tap a cell next to the path end to extend it.',
    rulesTxt2: 'A given number must be reached at exactly its step.',
    rulesTxt3: 'Tap a path cell to roll the path back to there.',
  },
  zh: {
    title: '像素连号',
    subtitle: '从 1 连到 N · 一条不断的路径',
    start: '开始',
    levelSelect: '选择关卡',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '路径完成！',
    next: '下一关',
    retry: '重玩',
    timeStr: '用时',
    rulesTxt1: '点击路径末端旁的格子来延伸路径。',
    rulesTxt2: '给定的数字必须在它对应的步数被走到。',
    rulesTxt3: '点击路径上的格子可把路径回退到那里。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-numpath:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-numpath:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
