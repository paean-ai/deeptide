// English / 中文 strings for Pixel Marble.

const STRINGS = {
  en: {
    title: 'PIXEL MARBLE',
    subtitle: 'Drag to tilt · steer the marble to the green pad',
    start: 'Start',
    levelSelect: 'Choose room',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Goal!',
    lose: 'Lost in a hole.',
    next: 'Next room',
    retry: 'Retry',
    timeStr: 'Time',
    drag: 'Drag anywhere to tilt',
    rulesTxt1: 'Drag anywhere — the drag direction is the tilt direction.',
    rulesTxt2: 'Walls bounce, holes swallow, the green pad clears the room.',
    rulesTxt3: 'Friction slows the marble; high speed builds momentum.',
  },
  zh: {
    title: '像素弹珠',
    subtitle: '拖动倾斜 · 把弹珠导向绿色终点',
    start: '开始',
    levelSelect: '选择关卡',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '到达终点！',
    lose: '掉进洞里了。',
    next: '下一关',
    retry: '再来',
    timeStr: '用时',
    drag: '拖动屏幕以倾斜',
    rulesTxt1: '在屏幕任意位置拖动，拖动方向即倾斜方向。',
    rulesTxt2: '撞墙反弹，掉洞失败，到达绿色终点过关。',
    rulesTxt3: '弹珠有摩擦，速度越快越难刹车。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-marble:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-marble:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
