// English / 中文 strings for Pixel Knife.

const STRINGS = {
  en: {
    title: 'PIXEL KNIFE',
    subtitle: 'Tap to throw · stick every knife without overlap',
    start: 'Start',
    levelSelect: 'Choose log',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Cleared!',
    lose: 'Snap.',
    next: 'Next log',
    retry: 'Retry',
    score: 'Score',
    left: 'Knives',
    apple: 'Apple +50',
    tap: 'TAP TO THROW',
    rulesTxt1: 'Tap anywhere to launch the next knife straight up.',
    rulesTxt2: 'Knife sticks where it meets the rim. Touching another knife snaps yours.',
    rulesTxt3: 'Land every knife in the log to clear; apples on the disk give +50.',
  },
  zh: {
    title: '像素飞刀',
    subtitle: '点击投刀，每把刀都钉上而不重叠',
    start: '开始',
    levelSelect: '选择木桩',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '过关！',
    lose: '崩断。',
    next: '下一根',
    retry: '再来',
    score: '分数',
    left: '剩余刀',
    apple: '苹果 +50',
    tap: '点击投刀',
    rulesTxt1: '点击屏幕投出下一把刀。',
    rulesTxt2: '刀钉在转盘上；与已有刀相撞则失败。',
    rulesTxt3: '钉完关卡的全部刀即过关；命中苹果加 50 分。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-knife:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-knife:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
