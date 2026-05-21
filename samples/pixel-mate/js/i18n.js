// English / 中文 strings for Pixel Mate.

const STRINGS = {
  en: {
    title: 'PIXEL MATE',
    subtitle: 'White to play and mate in one — find the move',
    start: 'Start',
    levelSelect: 'Choose puzzle',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Mate!',
    next: 'Next puzzle',
    retry: 'Retry',
    restart: 'Restart',
    attempts: 'Tries',
    rulesTxt1: 'Tap a White piece to select it; tap any legal target to move.',
    rulesTxt2: 'A wrong move undoes itself so you can keep trying.',
    rulesTxt3: 'Find the mate on the first try for three stars.',
  },
  zh: {
    title: '像素一招杀',
    subtitle: '白方一步将死 · 找到这步',
    start: '开始',
    levelSelect: '选择题目',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '将死！',
    next: '下一题',
    retry: '再来',
    restart: '重摆',
    attempts: '尝试',
    rulesTxt1: '点白棋选中，再点合法格落子。',
    rulesTxt2: '错招会自动撤回，可继续尝试。',
    rulesTxt3: '一次找到杀招即得三星。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-mate:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-mate:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
