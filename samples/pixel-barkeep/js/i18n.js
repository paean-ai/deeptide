// English / 中文 strings for Pixel Barkeep.

const STRINGS = {
  en: {
    title: 'PIXEL BARKEEP',
    subtitle: 'Serve the counters · hold the bar',
    start: 'Start',
    gameOver: 'Last call',
    score: 'Score',
    best: 'Best',
    round: 'Round',
    served: 'Served',
    retry: 'Retry',
    menu: 'Menu',
    rulesTxt1: 'Tap a counter to step there and slide a mug.',
    rulesTxt2: 'A mug shoves a patron back — off the end, they’re served.',
    rulesTxt3: 'Let a patron reach the bar and you lose a life.',
  },
  zh: {
    title: '像素酒保',
    subtitle: '服务吧台 · 守住柜台',
    start: '开始',
    gameOver: '打烊了',
    score: '分数',
    best: '最佳',
    round: '回合',
    served: '已服务',
    retry: '再来',
    menu: '菜单',
    rulesTxt1: '点击吧台，移动过去并滑出一杯酒。',
    rulesTxt2: '酒杯把客人推回——推出尽头即完成服务。',
    rulesTxt3: '客人冲到柜台，你就损失一条命。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-barkeep:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-barkeep:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
