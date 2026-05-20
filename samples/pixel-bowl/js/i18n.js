// English / 中文 strings for Pixel Bowl.

const STRINGS = {
  en: {
    title: 'PIXEL BOWL',
    subtitle: 'Drag UP from the ball · roll a perfect frame',
    start: 'Start',
    backToMenu: 'Menu',
    newGame: 'New game',
    rulesTxt1: 'Drag UP from the ball to set aim and power; release to roll.',
    rulesTxt2: '10 frames, two throws each. Strike (X) and spare (/) bonus.',
    rulesTxt3: 'Frame 10 grants up to a third throw on a strike or spare.',
    frame: 'F',
    best: 'Best',
    final: 'Final',
    again: 'Play again',
    aim: 'Swipe up to roll',
  },
  zh: {
    title: '像素保龄',
    subtitle: '从球向上拖动 · 滚出全中',
    start: '开始',
    backToMenu: '菜单',
    newGame: '新游戏',
    rulesTxt1: '从球向上拖动设定方向与力度，松手投球。',
    rulesTxt2: '十轮，每轮两投。Strike（X）/ Spare（/）有加分。',
    rulesTxt3: '第 10 轮 strike 或 spare 时可加投一次。',
    frame: '第',
    best: '最好',
    final: '总分',
    again: '再来一局',
    aim: '向上划动投球',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-bowl:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-bowl:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
