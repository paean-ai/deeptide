// English / 中文 strings for Pixel Nurikabe.

const STRINGS = {
  en: {
    title: 'PIXEL NURIKABE',
    subtitle: 'Shade the sea around numbered islands',
    start: 'Start',
    levelSelect: 'Choose puzzle',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Solved!',
    next: 'Next puzzle',
    retry: 'Retry',
    timeStr: 'Time',
    mistakes: 'Mistakes',
    rulesTxt1: 'Numbered cells start an island; the number is its size.',
    rulesTxt2: 'Different islands cannot touch in any orthogonal direction.',
    rulesTxt3: 'The remaining sea must be one piece with no 2x2 block.',
    rulesTxt4: 'Tap a blank cell to cycle: blank -> sea -> dot.',
  },
  zh: {
    title: '像素海岛',
    subtitle: '在数字岛屿周围涂出唯一的海面',
    start: '开始',
    levelSelect: '选择关卡',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '完成！',
    next: '下一关',
    retry: '再来一次',
    timeStr: '用时',
    mistakes: '错误',
    rulesTxt1: '数字所在格是岛屿起点，数字 = 岛屿格数。',
    rulesTxt2: '不同岛屿之间不能上下左右相邻。',
    rulesTxt3: '剩下的海要四连通，且不出现 2×2 全海方块。',
    rulesTxt4: '点空格循环：空 → 海 → 标白点。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-nurikabe:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-nurikabe:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
