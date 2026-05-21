// English / 中文 strings for Pixel One-Line.

const STRINGS = {
  en: {
    title: 'PIXEL ONE-LINE',
    subtitle: 'Trace every edge once · never lift the pen',
    start: 'Start',
    levelSelect: 'Choose figure',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Drawn in one stroke!',
    next: 'Next figure',
    retry: 'Retry',
    undo: 'Undo',
    restart: 'Restart',
    edges: 'Edges',
    rulesTxt1: 'Tap a node to place the pen, then tap a connected node to draw.',
    rulesTxt2: 'Every edge must be traced exactly once — no repeats.',
    rulesTxt3: 'Cover them all in one continuous line to win.',
  },
  zh: {
    title: '像素一笔画',
    subtitle: '每条线只画一次 · 笔不离纸',
    start: '开始',
    levelSelect: '选择图形',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '一笔完成！',
    next: '下一图',
    retry: '再来',
    undo: '撤销',
    restart: '重画',
    edges: '边',
    rulesTxt1: '点一个节点落笔，再点相连的节点画线。',
    rulesTxt2: '每条边必须且只能画一次，不能重复。',
    rulesTxt3: '一笔连续画完所有边即胜。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-oneline:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-oneline:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
