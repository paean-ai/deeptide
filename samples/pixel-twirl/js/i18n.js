// Pixel Twirl - English / Chinese strings.

const STRINGS = {
  en: {
    title: 'PIXEL TWIRL',
    subtitle: 'Spin the blocks. Rebuild the picture.',
    start: 'START',
    levelSelect: 'PICK A PICTURE',
    locked: 'LOCKED',
    cleared: 'CLEARED',
    back: 'MENU',
    moves: 'TWIRLS',
    par: 'PAR',
    target: 'TARGET',
    undo: 'UNDO',
    restart: 'RESTART',
    spin: 'SPIN',
    win: 'PICTURE RESTORED',
    retry: 'RETRY',
    next: 'NEXT',
    rules1: 'Tap a 2×2 junction — those four tiles spin a quarter turn.',
    rules2: 'Tap SPIN to flip the turn direction.',
    rules3: 'Rebuild the target picture. Match par for three stars.',
  },
  zh: {
    title: '像素旋格',
    subtitle: '旋转方块，还原图案',
    start: '开始',
    levelSelect: '选择图案',
    locked: '未解锁',
    cleared: '已通关',
    back: '菜单',
    moves: '旋转',
    par: '标准',
    target: '目标',
    undo: '撤销',
    restart: '重来',
    spin: '方向',
    win: '图案还原',
    retry: '重试',
    next: '下一关',
    rules1: '点击 2×2 交叉点，那四块会旋转 90 度',
    rules2: '点击「方向」切换旋转方向',
    rules3: '还原目标图案，达到标准步数得三星',
  },
};

let lang = 'en';
function loadLang() {
  try {
    const v = localStorage.getItem('pixel-twirl:lang');
    if (v === 'en' || v === 'zh') lang = v;
  } catch (e) { /* storage unavailable */ }
}
function saveLang() {
  try { localStorage.setItem('pixel-twirl:lang', lang); } catch (e) { /* ignore */ }
}
function t(key) { return STRINGS[lang][key]; }
function L(pair) { return pair[lang === 'zh' ? 1 : 0]; }
