// English / 中文 strings for Pixel Klotski.

const STRINGS = {
  en: {
    title: 'PIXEL KLOTSKI',
    subtitle: 'Slide the red general to the bottom-centre exit',
    start: 'Start',
    levelSelect: 'Choose puzzle',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'General escaped!',
    next: 'Next puzzle',
    retry: 'Retry',
    undo: 'Undo',
    restart: 'Restart',
    moves: 'Moves',
    par: 'Par',
    best: 'Best',
    rulesTxt1: 'Tap a block to select it; tap a side or arrow key to slide.',
    rulesTxt2: 'Blocks never rotate. They only slide one cell at a time into empty space.',
    rulesTxt3: 'Match the BFS par for three stars — the legendary Huarong board takes 116.',
  },
  zh: {
    title: '像素华容道',
    subtitle: '把红色大将滑到底部正中出口',
    start: '开始',
    levelSelect: '选择关卡',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '大将脱险！',
    next: '下一关',
    retry: '再来',
    undo: '撤销',
    restart: '重开',
    moves: '步数',
    par: '标准',
    best: '最少',
    rulesTxt1: '点击方块选中，再点击方向（或按方向键）滑动。',
    rulesTxt2: '方块不能旋转，每次只能朝相邻空格滑动一格。',
    rulesTxt3: '达到 BFS 最优解即三星——传统华容布局最优为 116 步。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-klotski:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-klotski:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
