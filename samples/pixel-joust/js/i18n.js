// English / 中文 strings for Pixel Joust.

const STRINGS = {
  en: {
    title: 'PIXEL JOUST',
    subtitle: 'Flap to fly · the higher lance wins · clear every rider',
    start: 'Start',
    levelSelect: 'Choose tourney',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Field cleared!',
    lose: 'Unhorsed',
    next: 'Next tourney',
    retry: 'Retry',
    score: 'Score',
    high: 'Best',
    lives: 'Lives',
    rulesTxt1: 'Tap ✦ to flap upward; ← / → buttons steer your ostrich.',
    rulesTxt2: 'When you cross a rival, the higher lance wins — the lower one falls.',
    rulesTxt3: 'Scoop up the dropped eggs before they hatch new buzzards.',
  },
  zh: {
    title: '像素长矛',
    subtitle: '扇翅飞行 · 长矛位置更高者胜 · 击落所有骑兵',
    start: '开始',
    levelSelect: '选择比武',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '清场完成！',
    lose: '坠地落马',
    next: '下一场',
    retry: '再来',
    score: '分数',
    high: '最佳',
    lives: '命',
    rulesTxt1: '点 ✦ 扇翅上升，← / → 控制鸵鸟横向移动。',
    rulesTxt2: '与对手交锋时，长矛较高的一方获胜。',
    rulesTxt3: '尽快捡走掉落的蛋，否则会孵出新的秃鹰骑士。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-joust:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-joust:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
