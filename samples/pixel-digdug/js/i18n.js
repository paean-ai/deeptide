// English / 中文 strings for Pixel DigDug.

const STRINGS = {
  en: {
    title: 'PIXEL DIGDUG',
    subtitle: 'Dig tunnels · pump enemies in your line · clear the pit',
    start: 'Start',
    levelSelect: 'Choose pit',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Pit cleared!',
    lose: 'Out of lives',
    next: 'Next pit',
    retry: 'Retry',
    score: 'Score',
    high: 'Best',
    lives: 'Lives',
    rulesTxt1: 'Tap the ←↓↑→ pad to dig in that direction.',
    rulesTxt2: 'Hold the ★ pump to inflate the enemy in your line of sight.',
    rulesTxt3: 'Three pumps pops it (+200). Squash one with a falling rock for +400.',
  },
  zh: {
    title: '像素挖掘工',
    subtitle: '挖掘隧道 · 视线打气敌人 · 清空坑道',
    start: '开始',
    levelSelect: '选择坑道',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '清空坑道！',
    lose: '没有命了',
    next: '下一关',
    retry: '再来',
    score: '分数',
    high: '最佳',
    lives: '命',
    rulesTxt1: '点击 ←↓↑→ 控制角色挖掘方向。',
    rulesTxt2: '按住 ★ 打气，使视线正对的敌人膨胀。',
    rulesTxt3: '三次打气即可爆破（+200）。落岩砸中敌人 +400。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-digdug:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-digdug:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
