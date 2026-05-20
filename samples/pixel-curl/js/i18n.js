// English / 中文 strings for Pixel Curl.

const STRINGS = {
  en: {
    title: 'PIXEL CURL',
    subtitle: 'Slide stones into the house · the closest wins the end',
    start: 'Start',
    levelSelect: 'Choose opponent',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'You take the end!',
    lose: 'Opponent steals.',
    tie: 'Blank end.',
    next: 'Next opponent',
    retry: 'Retry',
    aim: 'Drag DOWN from the stone, release to slide',
    yourTurn: 'YOUR THROW',
    aiTurn: 'OPPONENT',
    you: 'You',
    opp: 'Opp',
    rulesTxt1: 'Drag DOWN from the stone — release to slide forward.',
    rulesTxt2: 'Stones bounce off each other; nudge or take out the rival.',
    rulesTxt3: 'After 8 stones, you score one per closer stone than them.',
  },
  zh: {
    title: '像素冰壶',
    subtitle: '把冰壶滑入大本营 · 离中心最近者得分',
    start: '开始',
    levelSelect: '选择对手',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '你拿下这局！',
    lose: '对手反偷。',
    tie: '空局。',
    next: '下一对手',
    retry: '再来',
    aim: '从冰壶向下拖动，松手出手',
    yourTurn: '你的回合',
    aiTurn: '对手',
    you: '你',
    opp: '对手',
    rulesTxt1: '从冰壶向下拖动，松手向前滑行。',
    rulesTxt2: '冰壶会相互弹开；可用来撞掉对手的石头。',
    rulesTxt3: '八壶过后，比对手更靠近中心的连续石头都得 1 分。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-curl:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-curl:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
