// Pixel Aegis - English / Chinese strings.

const STRINGS = {
  en: {
    title: 'PIXEL AEGIS',
    subtitle: 'Drag the shield. Turn their fire back.',
    start: 'START',
    stageSelect: 'CHOOSE A SIEGE',
    locked: 'LOCKED',
    cleared: 'CLEARED',
    back: 'MENU',
    win: 'CORE HELD',
    lose: 'CORE BREACHED',
    retry: 'RETRY',
    next: 'NEXT',
    core: 'CORE',
    pulse: 'PULSE',
    ready: 'READY',
    rules1: 'Drag to swing the shield arc around the core.',
    rules2: 'A shot you block rebounds and strikes the shooter that fired it.',
    rules3: 'Tap PULSE for a full-circle flash on a cooldown. Clear every shooter.',
  },
  zh: {
    title: '像素神盾',
    subtitle: '转动护盾，以彼之矢还击',
    start: '开始',
    stageSelect: '选择围攻',
    locked: '未解锁',
    cleared: '已通关',
    back: '菜单',
    win: '核心守住',
    lose: '核心失守',
    retry: '重试',
    next: '下一关',
    core: '核心',
    pulse: '脉冲',
    ready: '就绪',
    rules1: '拖动以转动核心周围的护盾弧',
    rules2: '被挡下的弹会反弹，击中射出它的敌人',
    rules3: '点击「脉冲」释放全周护盾闪光（有冷却）。清空所有敌人',
  },
};

let lang = 'en';
function loadLang() {
  try {
    const v = localStorage.getItem('pixel-aegis:lang');
    if (v === 'en' || v === 'zh') lang = v;
  } catch (e) { /* storage unavailable */ }
}
function saveLang() {
  try { localStorage.setItem('pixel-aegis:lang', lang); } catch (e) { /* ignore */ }
}
function t(key) { return STRINGS[lang][key]; }
function L(pair) { return pair[lang === 'zh' ? 1 : 0]; }
