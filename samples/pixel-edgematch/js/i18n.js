// English / 中文 strings for Pixel Edgematch.

const STRINGS = {
  en: {
    title: 'PIXEL EDGEMATCH',
    subtitle: 'Swap and spin · match every edge',
    start: 'Start',
    levelSelect: 'Choose room',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'All edges met!',
    next: 'Next room',
    retry: 'Retry',
    moves: 'Moves',
    best: 'Best',
    rulesTxt1: 'Tap a tile to pick it up; tap another to swap them.',
    rulesTxt2: 'Tap the picked tile again to rotate it 90°.',
    rulesTxt3: 'Every shared edge must be the same colour on both sides.',
  },
  zh: {
    title: '像素拼边',
    subtitle: '交换与旋转 · 让每条边相接',
    start: '开始',
    levelSelect: '选择房间',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '全部相接！',
    next: '下一间',
    retry: '重玩',
    moves: '步数',
    best: '最佳',
    rulesTxt1: '点一块拿起，再点另一块即交换两者。',
    rulesTxt2: '再次点击拿起的那块可将它旋转 90°。',
    rulesTxt3: '相邻两块的接边必须颜色一致。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-edgematch:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-edgematch:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
