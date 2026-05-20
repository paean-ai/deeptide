// English / 中文 strings for Pixel 2048.

const STRINGS = {
  en: {
    title: 'PIXEL 2048',
    subtitle: 'Slide tiles · merge matching numbers · reach 2048',
    start: 'Start',
    backToMenu: 'Menu',
    newGame: 'New game',
    undo: 'Undo',
    keepPlaying: 'Keep going',
    score: 'Score',
    high: 'Best',
    moves: 'Moves',
    win: 'You reached 2048!',
    over: 'No moves left',
    rulesTxt1: 'Swipe or use the arrow keys to slide all tiles in a direction.',
    rulesTxt2: 'When two tiles of the same number touch they merge into double.',
    rulesTxt3: 'Reach the 2048 tile to win. Single-step undo is available.',
  },
  zh: {
    title: '像素 2048',
    subtitle: '滑动方块 · 相同数字合并 · 凑出 2048',
    start: '开始',
    backToMenu: '菜单',
    newGame: '新游戏',
    undo: '撤销',
    keepPlaying: '继续游戏',
    score: '分数',
    high: '最佳',
    moves: '步数',
    win: '凑出 2048！',
    over: '没有可以移动的方向',
    rulesTxt1: '滑动或方向键让所有方块朝一个方向滑动。',
    rulesTxt2: '相同数字的两块相遇即合并为双倍。',
    rulesTxt3: '凑出 2048 即胜。可撤销最近一步。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-2048:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-2048:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
