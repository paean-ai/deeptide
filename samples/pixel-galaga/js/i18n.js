// English / 中文 strings for Pixel Galaga.

const STRINGS = {
  en: {
    title: 'PIXEL GALAGA',
    subtitle: 'Hold the line · shoot the formation · dodge the dive bombs',
    start: 'Start',
    levelSelect: 'Choose wave',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Squadron cleared!',
    lose: 'Shot down',
    next: 'Next wave',
    retry: 'Retry',
    score: 'Score',
    high: 'Best',
    lives: 'Lives',
    rulesTxt1: 'Drag (or arrow keys) to slide your fighter; cannon auto-fires.',
    rulesTxt2: 'Formation kills score 50. A diving enemy scores 200 — bigger risk.',
    rulesTxt3: 'Dodge bullets and the dives themselves; 3 lives.',
  },
  zh: {
    title: '像素小蜂',
    subtitle: '坚守阵线 · 击落编队 · 躲开俯冲',
    start: '开始',
    levelSelect: '选择波次',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '中队清场！',
    lose: '被击落',
    next: '下一波',
    retry: '再来',
    score: '分数',
    high: '最佳',
    lives: '命',
    rulesTxt1: '拖动（或方向键）操控战机，炮塔自动开火。',
    rulesTxt2: '击中编队 +50；俯冲中的敌机 +200，风险更高。',
    rulesTxt3: '躲开子弹和俯冲机体，共 3 条命。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-galaga:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-galaga:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
