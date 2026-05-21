// Pixel Foray - English / Chinese strings.

const STRINGS = {
  en: {
    title: 'PIXEL FORAY',
    subtitle: 'Read the threat. Strike. Survive.',
    start: 'START',
    roomSelect: 'CHOOSE A ROOM',
    locked: 'LOCKED',
    cleared: 'CLEARED',
    back: 'MENU',
    restart: 'RESTART',
    win: 'ROOM CLEARED',
    lose: 'STRUCK DOWN',
    retry: 'RETRY',
    next: 'NEXT ROOM',
    foes: 'FOES',
    rules1: 'Tap a neighbouring tile to move; tap a foe to strike it.',
    rules2: 'Foes telegraph: a ghost marks where one steps, a red line is an archer’s shot. Red tiles will hurt you.',
    rules3: 'A killing blow cancels a foe’s turn; a wound does not. Clear every foe.',
  },
  zh: {
    title: '像素突袭',
    subtitle: '看清威胁，出击，求生',
    start: '开始',
    roomSelect: '选择房间',
    locked: '未解锁',
    cleared: '已通关',
    back: '菜单',
    restart: '重来',
    win: '房间清空',
    lose: '力战而亡',
    retry: '重试',
    next: '下一间',
    foes: '敌人',
    rules1: '点击相邻格移动，点击敌人将其攻击',
    rules2: '敌人会预示行动：幽影是它将走到的格，红线是弓手射击。红格会伤到你',
    rules3: '击杀可取消该敌人的回合，击伤则不行。清空所有敌人',
  },
};

let lang = 'en';
function loadLang() {
  try {
    const v = localStorage.getItem('pixel-foray:lang');
    if (v === 'en' || v === 'zh') lang = v;
  } catch (e) { /* storage unavailable */ }
}
function saveLang() {
  try { localStorage.setItem('pixel-foray:lang', lang); } catch (e) { /* ignore */ }
}
function t(key) { return STRINGS[lang][key]; }
function L(pair) { return pair[lang === 'zh' ? 1 : 0]; }
