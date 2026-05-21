// English / 中文 strings for Pixel Fit.

const STRINGS = {
  en: {
    title: 'PIXEL FIT',
    subtitle: 'Rotate and pack every piece',
    start: 'Start',
    levelSelect: 'Choose frame',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Frame filled!',
    next: 'Next frame',
    retry: 'Retry',
    moves: 'Moves',
    best: 'Best',
    rulesTxt1: 'Tap a tray piece to pick it up; tap it again to rotate.',
    rulesTxt2: 'Tap a frame cell to drop the piece there.',
    rulesTxt3: 'Tap a placed piece to lift it back out.',
    selHint: 'Tap the frame to place · tap the piece to rotate',
  },
  zh: {
    title: '像素拼框',
    subtitle: '旋转并填满每一块',
    start: '开始',
    levelSelect: '选择拼框',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '拼满了！',
    next: '下一框',
    retry: '重玩',
    moves: '步数',
    best: '最佳',
    rulesTxt1: '点托盘里的块拿起，再点一次旋转。',
    rulesTxt2: '点拼框里的格子放下这一块。',
    rulesTxt3: '点已放置的块可把它取回。',
    selHint: '点拼框放置 · 点该块旋转',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-fit:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-fit:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
