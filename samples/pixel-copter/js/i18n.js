// English / 中文 strings for Pixel Copter.

const STRINGS = {
  en: {
    title: 'PIXEL COPTER',
    subtitle: 'Hold to thrust · release to drop · ride the cave',
    start: 'Start',
    levelSelect: 'Choose cave',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Cave run',
    next: 'Next cave',
    retry: 'Retry',
    distance: 'Dist',
    best: 'Best',
    hold: 'HOLD TO FLY',
    rulesTxt1: 'Hold the screen to thrust the copter upward — release to fall.',
    rulesTxt2: 'Squeeze through the scrolling cave; pillars block the gap.',
    rulesTxt3: 'Speed and gap tighten the farther you fly. Score = distance.',
  },
  zh: {
    title: '像素直升机',
    subtitle: '按住上升·松手下降·驶过洞穴',
    start: '开始',
    levelSelect: '选择洞穴',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '洞穴飞行',
    next: '下一洞',
    retry: '再来',
    distance: '距离',
    best: '最好',
    hold: '按住飞行',
    rulesTxt1: '按住屏幕让直升机上升，松手下落。',
    rulesTxt2: '在滚动洞穴里穿行，柱体把通道隔得更窄。',
    rulesTxt3: '飞得越远速度越快、通道越窄。分数 = 距离。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-copter:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-copter:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
