// English / 中文 strings for Pixel Sumo.

const STRINGS = {
  en: {
    title: 'PIXEL SUMO',
    subtitle: 'Drag to dash · push your rival off the ring',
    start: 'Start',
    levelSelect: 'Choose rival',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Yusho!',
    lose: 'You got tossed.',
    next: 'Next rival',
    retry: 'Retry',
    aim: 'Drag from your wrestler to dash',
    rulesTxt1: 'Drag AWAY from your wrestler — release to dash forward.',
    rulesTxt2: 'Collisions are elastic; pile up momentum to send the rival flying.',
    rulesTxt3: 'First wrestler off the dohyō loses the bout.',
  },
  zh: {
    title: '像素相扑',
    subtitle: '拖动冲刺·把对手推出土俵',
    start: '开始',
    levelSelect: '选择对手',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '优胜！',
    lose: '被甩出去了。',
    next: '下一对手',
    retry: '再来',
    aim: '从自己向反方向拖动冲刺',
    rulesTxt1: '从自己向反方向拖动，松手向反向冲刺。',
    rulesTxt2: '弹性碰撞 — 越快撞，对手飞得越远。',
    rulesTxt3: '先掉出土俵者落败。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-sumo:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-sumo:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
