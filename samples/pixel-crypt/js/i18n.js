// English / 中文 strings for Pixel Crypt.

const STRINGS = {
  en: {
    title: 'PIXEL CRYPT',
    subtitle: 'Sweep the grid · slay every monster',
    start: 'Start',
    levelSelect: 'Choose floor',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Crypt cleared!',
    lose: 'You fell',
    next: 'Next floor',
    retry: 'Retry',
    lvl: 'Lv',
    reveal: 'REVEAL',
    flag: 'FLAG',
    rulesTxt1: 'Tap an empty tile — it shows the level-sum around it.',
    rulesTxt2: 'Tap a monster to fight it; a stronger foe wounds you.',
    rulesTxt3: 'Slay weak foes first to level up, then take the rest.',
  },
  zh: {
    title: '像素地宫',
    subtitle: '清扫格阵 · 斩尽妖物',
    start: '开始',
    levelSelect: '选择层数',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '通关！',
    lose: '你倒下了',
    next: '下一层',
    retry: '重来',
    lvl: '等级',
    reveal: '揭开',
    flag: '标记',
    rulesTxt1: '点空格——会显示其周围妖物的等级之和。',
    rulesTxt2: '点妖物即与其战斗，等级更高的妖物会伤到你。',
    rulesTxt3: '先斩弱敌升级，再去对付强敌。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-crypt:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-crypt:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
