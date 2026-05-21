// English / 中文 strings for Pixel Snowboard.

const STRINGS = {
  en: {
    title: 'PIXEL SNOWBOARD',
    subtitle: 'Carve the slope · thread the trees · reach the finish',
    start: 'Start',
    levelSelect: 'Choose slope',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Run cleared!',
    lose: 'Wiped out',
    next: 'Next slope',
    retry: 'Retry',
    score: 'Score',
    high: 'Best',
    lives: 'Lives',
    rulesTxt1: 'Drag (or arrow keys) to carve left and right.',
    rulesTxt2: 'Slip between the slalom flags for a bonus; hit ramps for a trick.',
    rulesTxt3: 'A trick hop clears trees; a tree at ground level wipes you out.',
  },
  zh: {
    title: '像素单板',
    subtitle: '雪道刻滑 · 穿林避树 · 冲过终点',
    start: '开始',
    levelSelect: '选择雪道',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '滑完全程！',
    lose: '摔倒了',
    next: '下一道',
    retry: '再来',
    score: '分数',
    high: '最佳',
    lives: '命',
    rulesTxt1: '拖动（或方向键）左右刻滑。',
    rulesTxt2: '穿过旗门获得奖励，撞上跳台做个空中动作。',
    rulesTxt3: '腾空时能越过树木；落地状态撞树即摔倒。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-snowboard:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-snowboard:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
