// English / 中文 strings for Pixel Archery.

const STRINGS = {
  en: {
    title: 'PIXEL ARCHERY',
    subtitle: 'Drag away from the bow · arc your arrow into the bullseye',
    start: 'Start',
    levelSelect: 'Choose range',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Round done',
    next: 'Next range',
    retry: 'Retry',
    score: 'Score',
    arrows: 'Arrows',
    wind: 'Wind',
    rulesTxt1: 'Drag AWAY from the bow — release to loose; the arrow arcs under gravity.',
    rulesTxt2: 'Wind pulls the arrow sideways. Bull 10 · Inner 8 · Mid 5 · Outer 3.',
    rulesTxt3: '10 arrows per range; clear a range to unlock the next.',
  },
  zh: {
    title: '像素弓道',
    subtitle: '从弓位反向拖动 · 让箭画一道弧线落入靶心',
    start: '开始',
    levelSelect: '选择箭场',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '回合结束',
    next: '下一场',
    retry: '再来',
    score: '分数',
    arrows: '箭',
    wind: '风',
    rulesTxt1: '从弓位反向拖动设定方向与力度，松手射出，箭在重力下成弧。',
    rulesTxt2: '横风让箭飞偏。靶心 10·内圈 8·中圈 5·外圈 3。',
    rulesTxt3: '每场 10 支箭，通过解锁下一关。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-archery:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-archery:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
