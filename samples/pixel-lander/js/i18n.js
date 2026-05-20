// English / 中文 strings for Pixel Lander.

const STRINGS = {
  en: {
    title: 'PIXEL LANDER',
    subtitle: 'Tap THRUST to fight gravity · land soft on the pad',
    start: 'Start',
    levelSelect: 'Choose mission',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    thrust: 'THRUST',
    left: '←',
    right: '→',
    fuel: 'Fuel',
    vy: 'vY',
    vx: 'vX',
    altitude: 'Alt',
    tilt: 'Tilt',
    score: 'Score',
    win: 'Touchdown!',
    lose: 'Crashed.',
    retry: 'Retry',
    next: 'Next mission',
    rulesTxt1: 'Down velocity below 28, drift below 18, tilt within ±14°.',
    rulesTxt2: 'Hit the landing pad - flat segment of the surface.',
    rulesTxt3: 'Watch fuel and wind. Score = fuel left · 2 + bonus.',
  },
  zh: {
    title: '像素登月舱',
    subtitle: '点 THRUST 抵消重力 · 平稳降落到平台',
    start: '开始',
    levelSelect: '选择任务',
    cleared: '已完成',
    locked: '未解锁',
    backToMenu: '菜单',
    thrust: '推进',
    left: '←',
    right: '→',
    fuel: '燃料',
    vy: '垂直',
    vx: '水平',
    altitude: '高度',
    tilt: '倾角',
    score: '分数',
    win: '成功着陆！',
    lose: '坠毁了。',
    retry: '重试',
    next: '下一任务',
    rulesTxt1: '垂直速度 <28、水平 <18、倾角 ±14° 内。',
    rulesTxt2: '降落点 = 地表的平整平台。',
    rulesTxt3: '注意燃料和横风。分数 = 余下燃料 ×2 + 奖励。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-lander:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-lander:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
