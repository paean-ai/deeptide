// English / 中文 strings for Pixel Idle Quest.

const STRINGS = {
  en: {
    title: 'PIXEL IDLE QUEST',
    subtitle: 'Tap to strike · idle to grind',
    start: 'Descend',
    stage: 'Stage',
    gold: 'Gold',
    relics: 'Relics',
    blade: 'Blade',
    squire: 'Squire',
    bladeDesc: '+ tap damage',
    squireDesc: '+ auto damage',
    tapDmg: 'Tap',
    dps: 'DPS',
    elite: 'ELITE',
    ascend: 'Ascend',
    ascendGet: 'relics',
    ascendLocked: 'Ascend at stage 10',
    offline: 'While away you ground out',
    rulesTxt1: 'Tap the monster to strike it.',
    rulesTxt2: 'Squires deal damage on their own — even idle.',
    rulesTxt3: 'Ascend to trade your run for relics that boost damage.',
  },
  zh: {
    title: '像素挂机远征',
    subtitle: '点击出击 · 挂机刷怪',
    start: '下潜',
    stage: '层数',
    gold: '金币',
    relics: '遗物',
    blade: '利刃',
    squire: '扈从',
    bladeDesc: '+ 点击伤害',
    squireDesc: '+ 自动伤害',
    tapDmg: '点击',
    dps: '每秒',
    elite: '精英',
    ascend: '飞升',
    ascendGet: '遗物',
    ascendLocked: '第10层可飞升',
    offline: '离线期间共刷得',
    rulesTxt1: '点击怪物即可攻击。',
    rulesTxt2: '扈从会自动造成伤害——挂机也照打。',
    rulesTxt3: '飞升可用本轮进度换取提升伤害的遗物。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-idle-quest:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-idle-quest:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
