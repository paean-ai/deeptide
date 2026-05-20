// English / 中文 strings for Pixel Tank.

const STRINGS = {
  en: {
    title: 'PIXEL TANK',
    subtitle: 'Wipe every enemy tank · defend the eagle',
    start: 'Start',
    levelSelect: 'Choose battle',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Stage clear!',
    lose: 'Base destroyed.',
    next: 'Next battle',
    retry: 'Retry',
    lives: 'Lives',
    left: 'Enemies',
    rulesTxt1: 'D-pad to move · FIRE to shoot one shell at a time.',
    rulesTxt2: 'Bricks crumble in one hit; steel walls block but never break.',
    rulesTxt3: 'Lose 3 tanks or let the eagle fall and the stage is lost.',
  },
  zh: {
    title: '像素坦克',
    subtitle: '消灭所有敌方坦克 · 守护我方鹰徽',
    start: '开始',
    levelSelect: '选择关卡',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '清场！',
    lose: '基地被毁。',
    next: '下一关',
    retry: '重试',
    lives: '生命',
    left: '剩敌',
    rulesTxt1: '方向键移动 · FIRE 一次发射一发炮弹。',
    rulesTxt2: '砖墙一发即碎；钢墙挡弹不破。',
    rulesTxt3: '丢失 3 辆坦克或鹰徽被毁则失败。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-tank:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-tank:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
