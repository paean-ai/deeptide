// English / 中文 strings for Pixel Centipede.

const STRINGS = {
  en: {
    title: 'PIXEL CENTIPEDE',
    subtitle: 'Shoot every centipede segment · split the worm · clear the field',
    start: 'Start',
    waveSelect: 'Choose wave',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Field cleared!',
    lose: 'Out of lives',
    next: 'Next wave',
    retry: 'Retry',
    score: 'Score',
    high: 'Best',
    lives: 'Lives',
    wave: 'Wave',
    rulesTxt1: 'Drag to move (bottom rows). Bullets fire automatically.',
    rulesTxt2: 'A hit segment turns into a mushroom and the worm splits.',
    rulesTxt3: 'Shoot the spider for a bonus — pick it off when it is close.',
  },
  zh: {
    title: '像素蜈蚣',
    subtitle: '射穿每一节 · 把虫切开 · 清空菇林',
    start: '开始',
    waveSelect: '选择关卡',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '菇林清空！',
    lose: '没有命了',
    next: '下一关',
    retry: '再来',
    score: '分数',
    high: '最佳',
    lives: '命',
    wave: '关卡',
    rulesTxt1: '拖动移动（仅限底部区域）。子弹自动开火。',
    rulesTxt2: '射中的节段变成蘑菇，蜈蚣会从该处断开。',
    rulesTxt3: '击落蜘蛛获得奖励——越靠近你击落分数越高。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-centipede:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-centipede:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
