// Pixel Tilt - English / Chinese strings.

const STRINGS = {
  en: {
    title: 'PIXEL TILT',
    subtitle: 'Tilt the cavern. Slide every crystal home.',
    start: 'START',
    levelSelect: 'PICK A CAVERN',
    locked: 'LOCKED',
    cleared: 'CLEARED',
    back: 'MENU',
    moves: 'TILTS',
    par: 'PAR',
    undo: 'UNDO',
    restart: 'RESTART',
    win: 'ALL CRYSTALS SET',
    retry: 'RETRY',
    next: 'NEXT',
    rules1: 'Swipe, tap an arrow, or use the arrow keys to tilt.',
    rules2: 'Every crystal slides until a wall, the edge or another crystal stops it.',
    rules3: 'Land each crystal on its matching goal. Match par for three stars.',
  },
  zh: {
    title: '像素滚晶',
    subtitle: '倾斜洞窟，让水晶各归其位',
    start: '开始',
    levelSelect: '选择洞窟',
    locked: '未解锁',
    cleared: '已通关',
    back: '菜单',
    moves: '次数',
    par: '标准',
    undo: '撤销',
    restart: '重来',
    win: '水晶归位',
    retry: '重试',
    next: '下一关',
    rules1: '滑动、点击方向键或用方向键来倾斜洞窟',
    rules2: '每颗水晶会滑动，直到撞上墙、边缘或另一颗水晶',
    rules3: '让每颗水晶停在同色目标上，达到标准次数得三星',
  },
};

let lang = 'en';
function loadLang() {
  try {
    const v = localStorage.getItem('pixel-tilt:lang');
    if (v === 'en' || v === 'zh') lang = v;
  } catch (e) { /* storage unavailable */ }
}
function saveLang() {
  try { localStorage.setItem('pixel-tilt:lang', lang); } catch (e) { /* ignore */ }
}
function t(key) { return STRINGS[lang][key]; }
function L(pair) { return pair[lang === 'zh' ? 1 : 0]; }
