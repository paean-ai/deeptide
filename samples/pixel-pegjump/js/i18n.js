// English / 中文 strings for Pixel Peg Jump.

const STRINGS = {
  en: {
    title: 'PIXEL PEG JUMP',
    subtitle: 'Jump over a peg into an empty hole · clear the board down to one peg',
    start: 'Start',
    levelSelect: 'Choose board',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'One peg left!',
    almost: 'No more moves',
    next: 'Next board',
    retry: 'Retry',
    undo: 'Undo',
    restart: 'Restart',
    pegsLeft: 'Pegs',
    moves: 'Moves',
    rulesTxt1: 'Tap a peg to select, then tap an empty hole two cells away.',
    rulesTxt2: 'You may jump only over an adjacent peg — the jumped peg is removed.',
    rulesTxt3: 'Solve every board down to a single peg for 3 stars.',
  },
  zh: {
    title: '像素跳棋',
    subtitle: '跳过相邻棋子落入空位 · 减到只剩一颗棋子',
    start: '开始',
    levelSelect: '选择关卡',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '仅剩一子！',
    almost: '无可行步数',
    next: '下一关',
    retry: '再来',
    undo: '撤销',
    restart: '重开',
    pegsLeft: '剩余',
    moves: '步数',
    rulesTxt1: '先点选一颗棋子，再点击两格之外的空位。',
    rulesTxt2: '必须跳过中间的相邻棋子；被跳过的棋子立即移除。',
    rulesTxt3: '减到只剩一颗即得三星。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-pegjump:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-pegjump:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
