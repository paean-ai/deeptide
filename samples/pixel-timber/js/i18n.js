// English / 中文 strings for Pixel Timber.

const STRINGS = {
  en: {
    title: 'PIXEL TIMBER',
    subtitle: 'Chop fast · dodge the branches',
    start: 'Start',
    tapToChop: 'Tap a side to chop',
    gameOver: 'Timber!',
    timeUp: 'Out of stamina',
    score: 'Score',
    best: 'Best',
    retry: 'Retry',
    menu: 'Menu',
    rulesTxt1: 'Tap the left or right side to chop the bottom log.',
    rulesTxt2: 'Each chop moves you to that side - dodge the branches.',
    rulesTxt3: 'Keep chopping: the stamina bar never stops draining.',
  },
  zh: {
    title: '像素伐木',
    subtitle: '快速劈砍 · 躲开树枝',
    start: '开始',
    tapToChop: '点击两侧劈砍',
    gameOver: '倒了！',
    timeUp: '体力耗尽',
    score: '得分',
    best: '最佳',
    retry: '再来',
    menu: '菜单',
    rulesTxt1: '点击左侧或右侧劈砍最下方的木段。',
    rulesTxt2: '每次劈砍都会把你移到那一侧——躲开树枝。',
    rulesTxt3: '保持劈砍：体力条一刻不停地下降。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-timber:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-timber:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
