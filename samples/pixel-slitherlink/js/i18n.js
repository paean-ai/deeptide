// English / 中文 strings for Pixel Slitherlink.

const STRINGS = {
  en: {
    title: 'PIXEL SLITHERLINK',
    subtitle: 'Draw one loop · obey the numbers',
    start: 'Start',
    levelSelect: 'Choose puzzle',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Loop closed!',
    next: 'Next puzzle',
    retry: 'Retry',
    timeStr: 'Time',
    rulesTxt1: 'Tap an edge: blank → line → cross → blank.',
    rulesTxt2: 'A number = loop segments touching that cell.',
    rulesTxt3: 'Lines must form ONE closed loop, no branches.',
  },
  zh: {
    title: '像素回环',
    subtitle: '画出唯一回路 · 满足数字',
    start: '开始',
    levelSelect: '选择关卡',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '回路闭合！',
    next: '下一关',
    retry: '重玩',
    timeStr: '用时',
    rulesTxt1: '点边线循环：空 → 线 → 叉 → 空。',
    rulesTxt2: '数字 = 该格四周的回路线段数。',
    rulesTxt3: '所有线必须连成唯一一条闭合回路，无分叉。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-slitherlink:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-slitherlink:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
