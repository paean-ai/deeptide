// English / 中文 strings for Pixel Frogger.

const STRINGS = {
  en: {
    title: 'PIXEL FROGGER',
    subtitle: 'Hop across the road and river · fill all five goal pads',
    start: 'Start',
    levelSelect: 'Choose level',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'All pads filled!',
    lose: 'Out of lives',
    next: 'Next level',
    retry: 'Retry',
    score: 'Score',
    high: 'Best',
    lives: 'Lives',
    time: 'Time',
    rulesTxt1: 'Tap a screen edge or swipe to hop one cell.',
    rulesTxt2: 'Ride logs and turtles across the river — bare water drowns you.',
    rulesTxt3: 'Land on each empty pad once. Cars and timeout cost a life.',
  },
  zh: {
    title: '像素青蛙过河',
    subtitle: '穿越马路与河流 · 填满五个终点池',
    start: '开始',
    levelSelect: '选择关卡',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '五池全满！',
    lose: '没有命了',
    next: '下一关',
    retry: '再来',
    score: '分数',
    high: '最佳',
    lives: '命',
    time: '时间',
    rulesTxt1: '点击屏幕边缘或滑动跳一格。',
    rulesTxt2: '骑乌龟和木头过河——掉进水里就淹死。',
    rulesTxt3: '依次填满每一个终点。撞车或超时损失一条命。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-frogger:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-frogger:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
