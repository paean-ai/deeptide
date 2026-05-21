// Pixel Splash - English / Chinese strings.

const STRINGS = {
  en: {
    title: 'PIXEL SPLASH',
    subtitle: 'Flood every cell with paint.',
    start: 'START',
    levelSelect: 'PICK A CANVAS',
    locked: 'LOCKED',
    cleared: 'CLEARED',
    back: 'MENU',
    moves: 'MOVES',
    par: 'PAR',
    undo: 'UNDO',
    restart: 'RESTART',
    win: 'CANVAS FILLED',
    lose: 'OUT OF PAINT',
    retry: 'RETRY',
    next: 'NEXT',
    rules1: 'Tap a paint to recolour your whole splash.',
    rules2: 'The splash swallows any patch of that colour it now touches.',
    rules3: 'Cover every cell before the budget runs out. Match par for three stars.',
  },
  zh: {
    title: '像素泼彩',
    subtitle: '用颜料铺满整幅画布',
    start: '开始',
    levelSelect: '选择画布',
    locked: '未解锁',
    cleared: '已通关',
    back: '菜单',
    moves: '步数',
    par: '标准',
    undo: '撤销',
    restart: '重来',
    win: '画布完成',
    lose: '颜料用尽',
    retry: '重试',
    next: '下一关',
    rules1: '点击颜料，将你的整块色块染色',
    rules2: '染色后会吞并所有相邻的同色区域',
    rules3: '在步数上限内铺满全部格子，达到标准步数得三星',
  },
};

let lang = 'en';
function loadLang() {
  try {
    const v = localStorage.getItem('pixel-splash:lang');
    if (v === 'en' || v === 'zh') lang = v;
  } catch (e) { /* storage unavailable */ }
}
function saveLang() {
  try { localStorage.setItem('pixel-splash:lang', lang); } catch (e) { /* ignore */ }
}
function t(key) { return STRINGS[lang][key]; }
