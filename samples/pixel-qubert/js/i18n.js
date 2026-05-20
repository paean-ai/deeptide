// English / 中文 strings for Pixel Qubert.

const STRINGS = {
  en: {
    title: 'PIXEL QUBERT',
    subtitle: 'Hop diagonally · paint every cube · dodge the falling balls',
    start: 'Start',
    levelSelect: 'Choose pyramid',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Pyramid painted!',
    lose: 'Out of lives',
    next: 'Next pyramid',
    retry: 'Retry',
    score: 'Score',
    high: 'Best',
    lives: 'Lives',
    cubes: 'Cubes',
    rulesTxt1: 'Tap above-left / above-right / below-left / below-right to hop.',
    rulesTxt2: 'Every landing tints the cube; finish every cube to clear the level.',
    rulesTxt3: 'Falling off the pyramid OR a ball-hit costs you a life.',
  },
  zh: {
    title: '像素方人',
    subtitle: '斜跳金字塔 · 染色每个立方 · 躲开滚落的红球',
    start: '开始',
    levelSelect: '选择金字塔',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '金字塔已染色！',
    lose: '没有命了',
    next: '下一关',
    retry: '再来',
    score: '分数',
    high: '最佳',
    lives: '命',
    cubes: '方块',
    rulesTxt1: '点击屏幕的左上 / 右上 / 左下 / 右下 进行斜跳。',
    rulesTxt2: '每次落地都给方块染色；染遍每个方块即通关。',
    rulesTxt3: '跌出金字塔或被球撞中都会损失一条命。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-qubert:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-qubert:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
