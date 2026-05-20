// English / 中文 strings for Pixel Pac-Pixel.

const STRINGS = {
  en: {
    title: 'PIXEL PAC-PIXEL',
    subtitle: 'Eat every pellet · avoid the ghosts · power-up to turn the tables',
    start: 'Start',
    levelSelect: 'Choose maze',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Maze cleared!',
    lose: 'Out of lives',
    next: 'Next maze',
    retry: 'Retry',
    score: 'Score',
    high: 'Best',
    lives: 'Lives',
    pellets: 'Pellets',
    rulesTxt1: 'Drag or swipe to choose direction (queued until the next corridor).',
    rulesTxt2: 'Eat a power-pellet at a corner to flip ghosts blue for a few seconds.',
    rulesTxt3: 'Catch a panicked ghost: +200. Get caught when chasing: -1 life.',
  },
  zh: {
    title: '像素吃豆豆',
    subtitle: '吃光所有豆 · 躲开幽灵 · 强化反击',
    start: '开始',
    levelSelect: '选择迷宫',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '迷宫通关！',
    lose: '没有命了',
    next: '下一关',
    retry: '再来',
    score: '分数',
    high: '最佳',
    lives: '命',
    pellets: '豆',
    rulesTxt1: '滑动或拖动选择方向（在下一个路口生效）。',
    rulesTxt2: '吃掉角落的强化豆，幽灵会短暂变蓝可被反吃。',
    rulesTxt3: '吃掉惊慌的幽灵：+200。被追上：-1 命。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-pacpixel:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-pacpixel:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
