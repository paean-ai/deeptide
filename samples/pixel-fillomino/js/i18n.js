// Pixel Fillomino - English / Chinese strings.

const STRINGS = {
  en: {
    title: 'PIXEL FILLOMINO',
    subtitle: 'Fill the grid. Every number is its region.',
    start: 'START',
    levelSelect: 'PICK A FIELD',
    locked: 'LOCKED',
    cleared: 'CLEARED',
    back: 'MENU',
    win: 'FIELD MAPPED',
    retry: 'RETRY',
    next: 'NEXT',
    erase: 'ERASE',
    restart: 'RESTART',
    left: 'LEFT',
    rules1: 'Every number N belongs to a block of exactly N joined cells.',
    rules2: 'Two blocks of the same size may not touch edge to edge.',
    rules3: 'Tap a cell, then a number. Fill the whole field to win.',
  },
  zh: {
    title: '像素填块',
    subtitle: '填满方格——数字即其区块大小',
    start: '开始',
    levelSelect: '选择田野',
    locked: '未解锁',
    cleared: '已通关',
    back: '菜单',
    win: '田野绘成',
    retry: '重来',
    next: '下一关',
    erase: '擦除',
    restart: '重置',
    left: '剩余',
    rules1: '数字 N 属于一个恰好 N 格相连的区块',
    rules2: '两个相同大小的区块不能边对边相邻',
    rules3: '点击格子再点数字，填满整片田野即获胜',
  },
};

let lang = 'en';
function loadLang() {
  try {
    const v = localStorage.getItem('pixel-fillomino:lang');
    if (v === 'en' || v === 'zh') lang = v;
  } catch (e) { /* storage unavailable */ }
}
function saveLang() {
  try { localStorage.setItem('pixel-fillomino:lang', lang); } catch (e) { /* ignore */ }
}
function t(key) { return STRINGS[lang][key]; }
function L(pair) { return pair[lang === 'zh' ? 1 : 0]; }
