// English / 中文 strings for Pixel Untangle.

const STRINGS = {
  en: {
    title: 'PIXEL UNTANGLE',
    subtitle: 'Drag the pegs · uncross every thread',
    start: 'Start',
    levelSelect: 'Choose puzzle',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Untangled!',
    next: 'Next puzzle',
    retry: 'Retry',
    timeStr: 'Time',
    crossings: 'CROSSINGS',
    solved: 'No crossings',
    rulesTxt1: 'Drag a peg to move it; threads follow.',
    rulesTxt2: 'Red threads are crossing another thread.',
    rulesTxt3: 'Untangle them all - a solution always exists.',
  },
  zh: {
    title: '像素解结',
    subtitle: '拖动木桩 · 让所有线不再相交',
    start: '开始',
    levelSelect: '选择关卡',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '解开了！',
    next: '下一关',
    retry: '重玩',
    timeStr: '用时',
    crossings: '交叉',
    solved: '无交叉',
    rulesTxt1: '拖动木桩即可移动，连线随之移动。',
    rulesTxt2: '红色的线表示它与另一条线相交。',
    rulesTxt3: '解开所有交叉——解一定存在。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-untangle:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-untangle:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
