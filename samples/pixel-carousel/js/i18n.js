// Pixel Carousel - English / Chinese strings.

const STRINGS = {
  en: {
    title: 'PIXEL CAROUSEL',
    subtitle: 'Spin the rows and columns. Rebuild the picture.',
    start: 'START',
    levelSelect: 'PICK A PICTURE',
    locked: 'LOCKED',
    cleared: 'CLEARED',
    back: 'MENU',
    moves: 'SHIFTS',
    par: 'PAR',
    target: 'TARGET',
    undo: 'UNDO',
    restart: 'RESTART',
    win: 'PICTURE RESTORED',
    retry: 'RETRY',
    next: 'NEXT',
    rules1: 'Swipe a row or column — the tiles cycle round and wrap edge to edge.',
    rules2: 'Every row and every column is a free carousel.',
    rules3: 'Rebuild the target picture. Match par for three stars.',
  },
  zh: {
    title: '像素旋盘',
    subtitle: '转动行与列，还原图案',
    start: '开始',
    levelSelect: '选择图案',
    locked: '未解锁',
    cleared: '已通关',
    back: '菜单',
    moves: '步数',
    par: '标准',
    target: '目标',
    undo: '撤销',
    restart: '重来',
    win: '图案还原',
    retry: '重试',
    next: '下一关',
    rules1: '滑动某一行或某一列，方块会循环并首尾相接',
    rules2: '每一行、每一列都是可自由转动的旋盘',
    rules3: '还原目标图案，达到标准步数得三星',
  },
};

let lang = 'en';
function loadLang() {
  try {
    const v = localStorage.getItem('pixel-carousel:lang');
    if (v === 'en' || v === 'zh') lang = v;
  } catch (e) { /* storage unavailable */ }
}
function saveLang() {
  try { localStorage.setItem('pixel-carousel:lang', lang); } catch (e) { /* ignore */ }
}
function t(key) { return STRINGS[lang][key]; }
function L(pair) { return pair[lang === 'zh' ? 1 : 0]; }
