// English / 中文 strings for Pixel Climb.

const STRINGS = {
  en: {
    title: 'PIXEL CLIMB',
    subtitle: 'Climb the beams · jump the barrels · reach the top',
    start: 'Start',
    levelSelect: 'Choose tower',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Princess rescued!',
    lose: 'Out of lives',
    next: 'Next tower',
    retry: 'Retry',
    score: 'Score',
    high: 'Best',
    lives: 'Lives',
    rulesTxt1: 'Tap the screen edges to move; tap UP/DOWN strips for ladders.',
    rulesTxt2: 'Tap JUMP to leap over a rolling barrel (+5 brave bonus).',
    rulesTxt3: 'Reach the top beam to clear the tower.',
  },
  zh: {
    title: '像素登塔',
    subtitle: '攀爬钢梁 · 跳过滚桶 · 登顶救出公主',
    start: '开始',
    levelSelect: '选择塔层',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '救出公主！',
    lose: '没有命了',
    next: '下一关',
    retry: '再来',
    score: '分数',
    high: '最佳',
    lives: '命',
    rulesTxt1: '点击屏幕边缘移动，上下条爬梯。',
    rulesTxt2: '点击 JUMP 跳过滚桶（+5 勇气奖励）。',
    rulesTxt3: '爬到顶部钢梁即通关。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-climb:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-climb:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
